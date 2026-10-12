// odb2mon - Vehicle Diagnostic Dashboard Client Controller

// 1. Safe Tauri API Discovery
function getTauriInvoke() {
  if (window.__TAURI__ && window.__TAURI__.core && typeof window.__TAURI__.core.invoke === 'function') {
    return window.__TAURI__.core.invoke;
  }
  if (window.__TAURI_INTERNALS__ && typeof window.__TAURI_INTERNALS__.invoke === 'function') {
    return window.__TAURI_INTERNALS__.invoke;
  }
  return null;
}

async function callBackend(command, args = {}) {
  const invokeFn = getTauriInvoke();
  if (invokeFn) {
    return await invokeFn(command, args);
  }
  return null;
}

function listenToEvent(eventName, callback) {
  if (window.__TAURI__ && window.__TAURI__.event && typeof window.__TAURI__.event.listen === 'function') {
    window.__TAURI__.event.listen(eventName, callback).catch(err => {
      console.warn(`Event listener registration failed for ${eventName}:`, err);
    });
  }
}

// Global State
let isMph = false;
let currentActiveDtcs = [];

// 2. Application Initializer
function initApp() {
  console.log('[odb2mon] Initializing dashboard UI controller...');

  // DOM References
  const navButtons = document.querySelectorAll('.nav-btn');
  const pages = document.querySelectorAll('.page');
  const valRpm = document.getElementById('valRpm');
  const valSpeed = document.getElementById('valSpeed');
  const valCoolant = document.getElementById('valCoolant');
  const valOil = document.getElementById('valOil');
  const unitSpeed = document.getElementById('unitSpeed');
  const rpmArc = document.getElementById('rpmArc');
  const speedArc = document.getElementById('speedArc');
  const coolantBar = document.getElementById('coolantBar');
  const oilBar = document.getElementById('oilBar');
  const oilUnsupportedBadge = document.getElementById('oilUnsupportedBadge');
  const statusDot = document.getElementById('statusDot');
  const statusText = document.getElementById('statusText');
  const dtcBadge = document.getElementById('dtcBadge');
  const dtcEmpty = document.getElementById('dtcEmpty');
  const dtcList = document.getElementById('dtcList');
  const confirmModal = document.getElementById('confirmModal');

  // Navigation Tab Switching
  navButtons.forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.preventDefault();
      const targetPage = btn.dataset.page;
      if (!targetPage) return;

      navButtons.forEach(b => b.classList.remove('active'));
      pages.forEach(p => p.classList.remove('active'));

      btn.classList.add('active');
      const targetElem = document.getElementById(`page-${targetPage}`);
      if (targetElem) {
        targetElem.classList.add('active');
      }
    });
  });

  // Unit Toggle (KM/H vs MPH)
  const toggleSpeedBtn = document.getElementById('toggleSpeedUnit');
  if (toggleSpeedBtn) {
    toggleSpeedBtn.addEventListener('click', () => {
      isMph = !isMph;
      if (unitSpeed) {
        unitSpeed.innerText = isMph ? 'MPH' : 'KM/H';
      }
    });
  }

  // Telemetry Renderer
  function renderTelemetry(snapshot) {
    if (!snapshot) return;

    // Connection Status
    const isConnected = snapshot.connection_status === 'Connected';
    if (statusDot) {
      statusDot.className = isConnected ? 'status-indicator' : 'status-indicator offline';
    }
    if (statusText) {
      statusText.innerText = snapshot.connection_status ? snapshot.connection_status.toUpperCase() : 'OFFLINE';
    }

    // Engine Crankshaft RPM (0 - 7000 scale)
    const rpm = snapshot.rpm !== null && snapshot.rpm !== undefined ? snapshot.rpm : 0;
    if (valRpm) {
      valRpm.innerText = rpm;
    }
    if (rpmArc) {
      const rpmPercent = Math.min(Math.max(rpm / 7000, 0), 1);
      const rpmOffset = 377 - (rpmPercent * 377);
      rpmArc.style.strokeDashoffset = rpmOffset;
    }

    // Vehicle Road Speed (0 - 240 km/h or 0 - 150 mph)
    let speed = snapshot.speed_kmh !== null && snapshot.speed_kmh !== undefined ? snapshot.speed_kmh : 0;
    if (isMph) {
      speed = Math.round(speed * 0.621371);
    }
    if (valSpeed) {
      valSpeed.innerText = speed;
    }
    if (speedArc) {
      const maxSpeed = isMph ? 150 : 240;
      const speedPercent = Math.min(Math.max(speed / maxSpeed, 0), 1);
      const speedOffset = 377 - (speedPercent * 377);
      speedArc.style.strokeDashoffset = speedOffset;
    }

    // Coolant Temperature
    if (snapshot.coolant_temp_c !== null && snapshot.coolant_temp_c !== undefined) {
      if (valCoolant) valCoolant.innerText = `${snapshot.coolant_temp_c.toFixed(1)} °C`;
      if (coolantBar) {
        const coolantPercent = Math.min(Math.max((snapshot.coolant_temp_c - 20) / 95, 0), 1) * 100;
        coolantBar.style.width = `${coolantPercent}%`;
      }
    } else {
      if (valCoolant) valCoolant.innerText = '-- °C';
      if (coolantBar) coolantBar.style.width = '0%';
    }

    // Engine Oil Temperature
    if (snapshot.oil_temp_supported) {
      if (oilUnsupportedBadge) oilUnsupportedBadge.style.display = 'none';
      if (valOil) valOil.style.display = 'block';
      if (snapshot.oil_temp_c !== null && snapshot.oil_temp_c !== undefined) {
        if (valOil) valOil.innerText = `${snapshot.oil_temp_c.toFixed(1)} °C`;
        if (oilBar) {
          const oilPercent = Math.min(Math.max((snapshot.oil_temp_c - 20) / 100, 0), 1) * 100;
          oilBar.style.width = `${oilPercent}%`;
        }
      } else {
        if (valOil) valOil.innerText = '-- °C';
        if (oilBar) oilBar.style.width = '0%';
      }
    } else {
      // Graceful degradation for unsupported oil temp (e.g. GMC Acadia Mode 22)
      if (valOil) valOil.style.display = 'none';
      if (oilBar) oilBar.style.width = '0%';
      if (oilUnsupportedBadge) oilUnsupportedBadge.style.display = 'block';
    }

    // Active Fault Codes
    currentActiveDtcs = snapshot.active_dtcs || [];
    updateDtcDisplay(currentActiveDtcs);
  }

  // Fault Codes List Renderer
  function updateDtcDisplay(dtcs) {
    if (dtcBadge) dtcBadge.innerText = dtcs.length;

    if (!dtcEmpty || !dtcList) return;

    if (dtcs.length === 0) {
      dtcEmpty.style.display = 'block';
      dtcList.style.display = 'none';
      dtcList.innerHTML = '';
    } else {
      dtcEmpty.style.display = 'none';
      dtcList.style.display = 'flex';
      dtcList.innerHTML = dtcs.map(dtc => `
        <div class="dtc-card">
          <div>
            <div class="dtc-code">${dtc.code}</div>
            <div class="dtc-desc">${dtc.description}</div>
          </div>
          <div class="dtc-badges">
            <span class="dtc-pill">${dtc.ecu_source || 'ECU'}</span>
            <span class="dtc-pill" style="color: var(--accent-red)">${dtc.status}</span>
          </div>
        </div>
      `).join('');
    }
  }

  // Scan Trouble Codes Button
  const btnScanDtcs = document.getElementById('btnScanDtcs');
  if (btnScanDtcs) {
    btnScanDtcs.addEventListener('click', async () => {
      try {
        const dtcs = await callBackend('read_fault_codes');
        if (dtcs) updateDtcDisplay(dtcs);
      } catch (err) {
        alert('Failed to scan DTCs: ' + err);
      }
    });
  }

  // Clear Fault Codes Flow
  const btnClearDtcs = document.getElementById('btnClearDtcs');
  if (btnClearDtcs) {
    btnClearDtcs.addEventListener('click', () => {
      if (confirmModal) confirmModal.style.display = 'flex';
    });
  }

  const btnCancelClear = document.getElementById('btnCancelClear');
  if (btnCancelClear) {
    btnCancelClear.addEventListener('click', () => {
      if (confirmModal) confirmModal.style.display = 'none';
    });
  }

  const btnConfirmClear = document.getElementById('btnConfirmClear');
  if (btnConfirmClear) {
    btnConfirmClear.addEventListener('click', async () => {
      try {
        await callBackend('clear_fault_codes');
        if (confirmModal) confirmModal.style.display = 'none';
        updateDtcDisplay([]);
        alert('Diagnostic trouble codes have been cleared from vehicle controller memory.');
      } catch (err) {
        alert('Error clearing codes: ' + err);
      }
    });
  }

  // Vehicle Profile Selector
  document.querySelectorAll('.btn-select-profile').forEach(btn => {
    btn.addEventListener('click', async () => {
      const profile = btn.dataset.profile;
      const bmwHost = document.getElementById('bmwHost') ? document.getElementById('bmwHost').value : '169.254.1.1';
      const bmwPort = document.getElementById('bmwPort') ? parseInt(document.getElementById('bmwPort').value, 10) : 6801;
      const gmcPort = document.getElementById('gmcPort') ? document.getElementById('gmcPort').value : 'COM3';
      const gmcBaud = document.getElementById('gmcBaud') ? parseInt(document.getElementById('gmcBaud').value, 10) : 38400;

      try {
        const res = await callBackend('switch_profile', {
          profileType: profile,
          enetHost: bmwHost,
          enetPort: bmwPort,
          btPort: gmcPort,
          btBaud: gmcBaud,
        });
        const vName = document.getElementById('vehicleName');
        if (vName && res) vName.innerText = res;
        // Navigate back to Dashboard tab
        const dashBtn = document.querySelector('.nav-btn[data-page="dashboard"]');
        if (dashBtn) dashBtn.click();
      } catch (err) {
        alert('Failed to switch profile: ' + err);
      }
    });
  });

  // In-App Update Checker
  const btnCheckUpdate = document.getElementById('btnCheckUpdate');
  if (btnCheckUpdate) {
    btnCheckUpdate.addEventListener('click', async () => {
      const uStatus = document.getElementById('updateStatusText');
      const pill = document.getElementById('updatePill');
      const details = document.getElementById('updateDetails');
      const notes = document.getElementById('updateReleaseNotes');
      const links = document.getElementById('updateLinks');

      if (uStatus) uStatus.innerText = 'Checking GitHub Releases API...';

      try {
        const info = await callBackend('check_update');
        if (info && info.update_available) {
          if (pill) {
            pill.innerText = `NEW VERSION: v${info.latest_version}`;
            pill.style.background = '#2a1515';
            pill.style.color = '#ff8a80';
            pill.style.borderColor = '#d50000';
          }
          if (uStatus) uStatus.innerText = `An upgrade is available (Current: v${info.current_version})`;
          if (details) details.style.display = 'block';
          if (notes) notes.innerText = info.release_notes || 'No release notes provided.';

          let html = '';
          if (info.msi_download_url) {
            html += `<a href="${info.msi_download_url}" target="_blank">DOWNLOAD .MSI INSTALLER</a>`;
          }
          if (info.exe_download_url) {
            html += `<a href="${info.exe_download_url}" target="_blank">DOWNLOAD .EXE</a>`;
          }
          if (links) links.innerHTML = html;
        } else if (info) {
          if (pill) pill.innerText = 'UP TO DATE';
          if (uStatus) uStatus.innerText = `You are running the latest version (v${info.current_version})`;
          if (details) details.style.display = 'none';
        }
      } catch (err) {
        if (uStatus) uStatus.innerText = 'Update check error: ' + err;
      }
    });
  }

  // Event Push Listener
  listenToEvent('telemetry-update', event => {
    if (event && event.payload) {
      renderTelemetry(event.payload);
    }
  });

  // Polling Loop (10 Hz): Queries IPC get_snapshot
  // Guarantees continuous live telemetry updates regardless of event bridge state
  setInterval(async () => {
    try {
      const snap = await callBackend('get_snapshot');
      if (snap && snap.connection_status) {
        renderTelemetry(snap);
      }
    } catch (_) {}
  }, 100);

  // Initial immediate snapshot query
  callBackend('get_snapshot').then(snap => {
    if (snap) renderTelemetry(snap);
  }).catch(() => {});
}

// Ensure execution when DOM is ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initApp);
} else {
  initApp();
}

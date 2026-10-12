// odb2mon - Vehicle Diagnostic Dashboard Client Controller

const isTauri = !!(window.__TAURI__ && window.__TAURI__.core);
const invoke = isTauri ? window.__TAURI__.core.invoke : async () => ({});
const listen = isTauri ? window.__TAURI__.event.listen : () => {};

let isMph = false;
let currentActiveDtcs = [];

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

// 1. Navigation Tab Switching
navButtons.forEach(btn => {
  btn.addEventListener('click', () => {
    const targetPage = btn.dataset.page;
    navButtons.forEach(b => b.classList.remove('active'));
    pages.forEach(p => p.classList.remove('active'));

    btn.classList.add('active');
    document.getElementById(`page-${targetPage}`).classList.add('active');
  });
});

// 2. Unit Toggle (KM/H vs MPH)
document.getElementById('toggleSpeedUnit').addEventListener('click', () => {
  isMph = !isMph;
  unitSpeed.innerText = isMph ? 'MPH' : 'KM/H';
});

// 3. Telemetry Update Handler
function renderTelemetry(snapshot) {
  if (!snapshot) return;

  // Status
  const isConnected = snapshot.connection_status === 'Connected';
  statusDot.className = isConnected ? 'status-indicator' : 'status-indicator offline';
  statusText.innerText = snapshot.connection_status ? snapshot.connection_status.toUpperCase() : 'OFFLINE';

  // RPM Gauge (Max 7000)
  const rpm = snapshot.rpm || 0;
  valRpm.innerText = rpm;
  const rpmPercent = Math.min(Math.max(rpm / 7000, 0), 1);
  const rpmOffset = 377 - (rpmPercent * 377);
  rpmArc.style.strokeDashoffset = rpmOffset;

  // Speed Gauge (Max 240 km/h or 150 mph)
  let speed = snapshot.speed_kmh || 0;
  if (isMph) {
    speed = Math.round(speed * 0.621371);
  }
  valSpeed.innerText = speed;
  const maxSpeed = isMph ? 150 : 240;
  const speedPercent = Math.min(Math.max(speed / maxSpeed, 0), 1);
  const speedOffset = 377 - (speedPercent * 377);
  speedArc.style.strokeDashoffset = speedOffset;

  // Coolant Temperature
  if (snapshot.coolant_temp_c !== null && snapshot.coolant_temp_c !== undefined) {
    valCoolant.innerText = `${snapshot.coolant_temp_c.toFixed(1)} °C`;
    const coolantPercent = Math.min(Math.max((snapshot.coolant_temp_c - 20) / 95, 0), 1) * 100;
    coolantBar.style.width = `${coolantPercent}%`;
  } else {
    valCoolant.innerText = '-- °C';
    coolantBar.style.width = '0%';
  }

  // Oil Temperature
  if (snapshot.oil_temp_supported) {
    oilUnsupportedBadge.style.display = 'none';
    valOil.style.display = 'block';
    if (snapshot.oil_temp_c !== null && snapshot.oil_temp_c !== undefined) {
      valOil.innerText = `${snapshot.oil_temp_c.toFixed(1)} °C`;
      const oilPercent = Math.min(Math.max((snapshot.oil_temp_c - 20) / 100, 0), 1) * 100;
      oilBar.style.width = `${oilPercent}%`;
    } else {
      valOil.innerText = '-- °C';
      oilBar.style.width = '0%';
    }
  } else {
    // Graceful degradation for unsupported oil temp (e.g. GMC Acadia)
    valOil.style.display = 'none';
    oilBar.style.width = '0%';
    oilUnsupportedBadge.style.display = 'block';
  }

  // Active DTCs
  currentActiveDtcs = snapshot.active_dtcs || [];
  updateDtcDisplay(currentActiveDtcs);
}

// 4. DTC Display Renderer
function updateDtcDisplay(dtcs) {
  dtcBadge.innerText = dtcs.length;

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

// 5. DTC Actions
document.getElementById('btnScanDtcs').addEventListener('click', async () => {
  try {
    const dtcs = await invoke('read_fault_codes');
    updateDtcDisplay(dtcs);
  } catch (err) {
    alert('Failed to scan DTCs: ' + err);
  }
});

document.getElementById('btnClearDtcs').addEventListener('click', () => {
  confirmModal.style.display = 'flex';
});

document.getElementById('btnCancelClear').addEventListener('click', () => {
  confirmModal.style.display = 'none';
});

document.getElementById('btnConfirmClear').addEventListener('click', async () => {
  try {
    await invoke('clear_fault_codes');
    confirmModal.style.display = 'none';
    updateDtcDisplay([]);
    alert('Diagnostic trouble codes have been cleared from vehicle controller memory.');
  } catch (err) {
    alert('Error clearing codes: ' + err);
  }
});

// 6. Profile Switching
document.querySelectorAll('.btn-select-profile').forEach(btn => {
  btn.addEventListener('click', async () => {
    const profile = btn.dataset.profile;
    const bmwHost = document.getElementById('bmwHost').value;
    const bmwPort = parseInt(document.getElementById('bmwPort').value, 10);
    const gmcPort = document.getElementById('gmcPort').value;
    const gmcBaud = parseInt(document.getElementById('gmcBaud').value, 10);

    try {
      const res = await invoke('switch_profile', {
        profileType: profile,
        enetHost: bmwHost,
        enetPort: bmwPort,
        btPort: gmcPort,
        btBaud: gmcBaud,
      });
      document.getElementById('vehicleName').innerText = res;
      // Switch back to dashboard
      document.querySelector('.nav-btn[data-page="dashboard"]').click();
    } catch (err) {
      alert('Failed to switch profile: ' + err);
    }
  });
});

// 7. Update Checker
document.getElementById('btnCheckUpdate').addEventListener('click', async () => {
  const statusText = document.getElementById('updateStatusText');
  const pill = document.getElementById('updatePill');
  const details = document.getElementById('updateDetails');
  const notes = document.getElementById('updateReleaseNotes');
  const links = document.getElementById('updateLinks');

  statusText.innerText = 'Checking GitHub Releases API...';

  try {
    const info = await invoke('check_update');
    if (info.update_available) {
      pill.innerText = `NEW VERSION: v${info.latest_version}`;
      pill.style.background = '#2a1515';
      pill.style.color = '#ff8a80';
      pill.style.borderColor = '#d50000';
      statusText.innerText = `An upgrade is available (Current: v${info.current_version})`;

      details.style.display = 'block';
      notes.innerText = info.release_notes || 'No release notes provided.';

      let html = '';
      if (info.msi_download_url) {
        html += `<a href="${info.msi_download_url}" target="_blank">DOWNLOAD .MSI INSTALLER</a>`;
      }
      if (info.exe_download_url) {
        html += `<a href="${info.exe_download_url}" target="_blank">DOWNLOAD .EXE</a>`;
      }
      links.innerHTML = html;
    } else {
      pill.innerText = 'UP TO DATE';
      statusText.innerText = `You are running the latest version (v${info.current_version})`;
      details.style.display = 'none';
    }
  } catch (err) {
    statusText.innerText = 'Update check error: ' + err;
  }
});

// 8. Initialization & Real-Time Event Listener
if (isTauri) {
  listen('telemetry-update', event => {
    renderTelemetry(event.payload);
  });

  // Initial fetch
  invoke('get_snapshot').then(snap => renderTelemetry(snap)).catch(() => {});
} else {
  // Offline browser development mock loop
  let simRpm = 800;
  setInterval(() => {
    simRpm = 800 + Math.floor(Math.sin(Date.now() / 1000) * 1800 + 1800);
    renderTelemetry({
      connection_status: 'Connected',
      rpm: simRpm,
      speed_kmh: Math.floor(simRpm / 35),
      coolant_temp_c: 88.5,
      oil_temp_c: 98.2,
      oil_temp_supported: true,
      active_dtcs: [
        { code: '120408', description: 'Charging pressure control: switch-off', status: 'Confirmed', ecu_source: 'DME' }
      ]
    });
  }, 100);
}

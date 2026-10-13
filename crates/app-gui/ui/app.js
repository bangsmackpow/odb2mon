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

  // Topology View References
  const btnScanTopology = document.getElementById('btnScanTopology');
  const topologyContainer = document.getElementById('topologyContainer');
  const topoTotalScanned = document.getElementById('topoTotalScanned');
  const topoOnlineCount = document.getElementById('topoOnlineCount');
  const topoFaultCount = document.getElementById('topoFaultCount');
  const topoHealthStatus = document.getElementById('topoHealthStatus');
  const topoScanLoading = document.getElementById('topoScanLoading');
  const topologyUnsupportedBanner = document.getElementById('topologyUnsupportedBanner');
  const topoMetricsBar = document.getElementById('topoMetricsBar');

  const ecuNodeModal = document.getElementById('ecuNodeModal');
  const ecuModalTitle = document.getElementById('ecuModalTitle');
  const ecuModalFullName = document.getElementById('ecuModalFullName');
  const ecuModalBadge = document.getElementById('ecuModalBadge');
  const ecuModalBus = document.getElementById('ecuModalBus');
  const ecuModalAddress = document.getElementById('ecuModalAddress');
  const ecuModalLatency = document.getElementById('ecuModalLatency');
  const ecuModalDtcCount = document.getElementById('ecuModalDtcCount');
  const ecuModalDtcList = document.getElementById('ecuModalDtcList');
  const btnCloseEcuModal = document.getElementById('btnCloseEcuModal');

  // Actuators References
  const btnStartBleed = document.getElementById('btnStartBleed');
  const btnStopBleed = document.getElementById('btnStopBleed');
  const bleedProgressFill = document.getElementById('bleedProgressFill');
  const bleedProgressVal = document.getElementById('bleedProgressVal');
  const bleedStageName = document.getElementById('bleedStageName');
  const bleedPumpSpeed = document.getElementById('bleedPumpSpeed');
  const bleedElapsed = document.getElementById('bleedElapsed');
  const bleedRemaining = document.getElementById('bleedRemaining');
  const bleedStatusText = document.getElementById('bleedStatusText');
  const waterPumpModal = document.getElementById('waterPumpModal');
  const btnCancelBleed = document.getElementById('btnCancelBleed');
  const btnConfirmBleed = document.getElementById('btnConfirmBleed');

  const transTempC = document.getElementById('transTempC');
  const transTempF = document.getElementById('transTempF');
  const transWindowBadge = document.getElementById('transWindowBadge');
  const transStatusMsg = document.getElementById('transStatusMsg');
  const btnStartTransFill = document.getElementById('btnStartTransFill');
  const transRoutineStatus = document.getElementById('transRoutineStatus');

  const fanDutyVal = document.getElementById('fanDutyVal');
  const fanStatusText = document.getElementById('fanStatusText');
  const btnFanSteps = document.querySelectorAll('.btn-fan-step');

  const flapStateVal = document.getElementById('flapStateVal');
  const flapStatusText = document.getElementById('flapStatusText');
  const btnOpenFlap = document.getElementById('btnOpenFlap');
  const btnCloseFlap = document.getElementById('btnCloseFlap');
  const btnRefreshActuators = document.getElementById('btnRefreshActuators');

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
      if (targetPage === 'topology') {
        fetchTopologyScan();
      }
      if (targetPage === 'service') {
        fetchBatteryStatus();
        fetchCbsReport();
        fetchActuatorsStatus();
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

  // BMW Service & Battery Telemetry Controller
  const valBatSoc = document.getElementById('valBatSoc');
  const batSocBar = document.getElementById('batSocBar');
  const valBatVoltage = document.getElementById('valBatVoltage');
  const valBatCurrent = document.getElementById('valBatCurrent');
  const valBatTemp = document.getElementById('valBatTemp');
  const valBatType = document.getElementById('valBatType');
  const batteryUnsupportedBanner = document.getElementById('batteryUnsupportedBanner');
  const batteryServiceGrid = document.getElementById('batteryServiceGrid');
  const batteryModal = document.getElementById('batteryModal');
  const batteryRegStatus = document.getElementById('batteryRegStatus');
  const batteryHistorySummary = document.getElementById('batteryHistorySummary');

  async function fetchBatteryStatus() {
    try {
      const bat = await callBackend('get_battery_status');
      if (!bat) return;

      if (!bat.supported) {
        if (batteryUnsupportedBanner) batteryUnsupportedBanner.style.display = 'block';
        if (batteryServiceGrid) batteryServiceGrid.style.opacity = '0.5';
        return;
      }

      if (batteryUnsupportedBanner) batteryUnsupportedBanner.style.display = 'none';
      if (batteryServiceGrid) batteryServiceGrid.style.opacity = '1';

      // SoC
      const soc = Math.round(bat.state_of_charge_percent);
      if (valBatSoc) valBatSoc.innerText = `${soc} %`;
      if (batSocBar) {
        batSocBar.style.width = `${Math.min(Math.max(soc, 0), 100)}%`;
        if (soc < 50) {
          batSocBar.style.background = 'var(--accent-red)';
        } else if (soc < 70) {
          batSocBar.style.background = 'var(--accent-amber)';
        } else {
          batSocBar.style.background = 'linear-gradient(90deg, #ff9800, #00e676)';
        }
      }

      // Voltage
      if (valBatVoltage) {
        valBatVoltage.innerText = `${bat.voltage.toFixed(2)} V`;
      }

      // Net Current
      if (valBatCurrent) {
        const sign = bat.current_amps > 0 ? '+' : '';
        valBatCurrent.innerText = `${sign}${bat.current_amps.toFixed(1)} A`;
        valBatCurrent.style.color = bat.current_amps >= 0 ? 'var(--accent-green)' : '#ff8a80';
      }

      // Temperature
      if (valBatTemp) {
        valBatTemp.innerText = `${bat.temperature_celsius.toFixed(1)} °C`;
      }

      // Capacity & Type
      if (valBatType) {
        const cap = bat.capacity_ah ? `${bat.capacity_ah} Ah` : '90 Ah';
        const type = bat.battery_type || 'AGM';
        valBatType.innerText = `${cap} · ${type}`;
      }

      // History
      const history = bat.last_replacement_km || [];
      const valHist1 = document.getElementById('valHist1');
      const valHistMiles1 = document.getElementById('valHistMiles1');
      const valHist2 = document.getElementById('valHist2');
      const valHistMiles2 = document.getElementById('valHistMiles2');
      const valHist3 = document.getElementById('valHist3');
      const valHistMiles3 = document.getElementById('valHistMiles3');

      if (valHist1) {
        if (history.length > 0 && history[0] > 0) {
          valHist1.innerText = `${history[0].toLocaleString()} km`;
          if (valHistMiles1) valHistMiles1.innerText = `~${Math.round(history[0] * 0.621371).toLocaleString()} mi`;
        } else {
          valHist1.innerText = '-- km';
          if (valHistMiles1) valHistMiles1.innerText = '-- mi';
        }
      }

      if (valHist2) {
        if (history.length > 1 && history[1] > 0) {
          valHist2.innerText = `${history[1].toLocaleString()} km`;
          if (valHistMiles2) valHistMiles2.innerText = `~${Math.round(history[1] * 0.621371).toLocaleString()} mi`;
        } else {
          valHist2.innerText = '-- km';
          if (valHistMiles2) valHistMiles2.innerText = '-- mi';
        }
      }

      if (valHist3) {
        if (history.length > 2 && history[2] > 0) {
          valHist3.innerText = `${history[2].toLocaleString()} km`;
          if (valHistMiles3) valHistMiles3.innerText = `~${Math.round(history[2] * 0.621371).toLocaleString()} mi`;
        } else {
          valHist3.innerText = '-- km';
          if (valHistMiles3) valHistMiles3.innerText = '-- mi';
        }
      }

      if (batteryHistorySummary) {
        if (history.length > 0) {
          batteryHistorySummary.innerText = `Last registration confirmed at ${history[0].toLocaleString()} km. DME charging profile calibrated.`;
        } else {
          batteryHistorySummary.innerText = 'No prior battery replacement registrations logged in DME.';
        }
      }
    } catch (err) {
      if (batteryUnsupportedBanner) batteryUnsupportedBanner.style.display = 'block';
      if (batteryServiceGrid) batteryServiceGrid.style.opacity = '0.5';
    }
  }

  // Hook up button handlers
  const btnRefreshBattery = document.getElementById('btnRefreshBattery');
  if (btnRefreshBattery) {
    btnRefreshBattery.addEventListener('click', () => {
      fetchBatteryStatus();
    });
  }

  const btnRegisterBattery = document.getElementById('btnRegisterBattery');
  if (btnRegisterBattery) {
    btnRegisterBattery.addEventListener('click', () => {
      if (batteryModal) batteryModal.style.display = 'flex';
    });
  }

  const btnCancelBatteryReg = document.getElementById('btnCancelBatteryReg');
  if (btnCancelBatteryReg) {
    btnCancelBatteryReg.addEventListener('click', () => {
      if (batteryModal) batteryModal.style.display = 'none';
    });
  }

  const btnConfirmBatteryReg = document.getElementById('btnConfirmBatteryReg');
  if (btnConfirmBatteryReg) {
    btnConfirmBatteryReg.addEventListener('click', async () => {
      try {
        if (batteryRegStatus) batteryRegStatus.innerText = 'Executing Routine 0x0219...';
        const msg = await callBackend('register_battery');
        if (batteryModal) batteryModal.style.display = 'none';
        if (batteryRegStatus) batteryRegStatus.innerText = '✓ Success: Battery registered in DME!';
        await fetchBatteryStatus();
        alert(msg || 'Battery registration successful!');
      } catch (err) {
        if (batteryModal) batteryModal.style.display = 'none';
        if (batteryRegStatus) batteryRegStatus.innerText = 'Error: ' + err;
        alert('Failed to register battery: ' + err);
      }
    });
  }

  // Condition Based Service (CBS) Controller
  const cbsItemsGrid = document.getElementById('cbsItemsGrid');
  const btnRefreshCbs = document.getElementById('btnRefreshCbs');
  const cbsModal = document.getElementById('cbsModal');
  const cbsModalItemText = document.getElementById('cbsModalItemText');
  const btnCancelCbsReset = document.getElementById('btnCancelCbsReset');
  const btnConfirmCbsReset = document.getElementById('btnConfirmCbsReset');
  let currentCbsResetItem = null;

  function getItemIcon(itemType) {
    switch (itemType) {
      case 'engine_oil': return '🛢️';
      case 'front_brakes': return '🛑';
      case 'rear_brakes': return '🛑';
      case 'brake_fluid': return '💧';
      case 'vehicle_check': return '🔍';
      case 'spark_plugs': return '⚡';
      default: return '🔧';
    }
  }

  async function fetchCbsReport() {
    if (!cbsItemsGrid) return;
    try {
      const report = await callBackend('get_cbs_report');
      if (!report || !report.supported || !report.items) {
        cbsItemsGrid.innerHTML = `
          <div class="cbs-card" style="grid-column: 1 / -1; text-align: center; color: var(--text-muted);">
            CBS monitoring is only supported on BMW UDS profiles.
          </div>
        `;
        return;
      }

      cbsItemsGrid.innerHTML = report.items.map(item => {
        const icon = getItemIcon(item.item_type);
        const percent = Math.round(item.remaining_percent);
        let statusClass = 'status-ok';
        let statusText = 'OK';
        let barColor = 'var(--accent-green)';

        if (item.status === 'warning') {
          statusClass = 'status-warning';
          statusText = 'DUE SOON';
          barColor = 'var(--accent-amber)';
        } else if (item.status === 'overdue') {
          statusClass = 'status-overdue';
          statusText = 'OVERDUE';
          barColor = 'var(--accent-red)';
        }

        const kmText = item.remaining_km !== null && item.remaining_km !== undefined
          ? `${item.remaining_km.toLocaleString()} km (~${Math.round(item.remaining_km * 0.621371).toLocaleString()} mi)`
          : 'Interval by Date';

        const dateText = item.due_date || 'Calculated by Km';

        return `
          <div class="cbs-card">
            <div class="cbs-card-top">
              <div class="cbs-title-group">
                <span class="cbs-icon">${icon}</span>
                <span class="cbs-name">${item.name}</span>
              </div>
              <span class="cbs-status-pill ${statusClass}">${statusText}</span>
            </div>

            <div class="cbs-life-container">
              <div class="cbs-life-header">
                <span>REMAINING SERVICE LIFE</span>
                <span class="cbs-life-val">${percent} %</span>
              </div>
              <div class="cbs-bar-bg">
                <div class="cbs-bar-fill" style="width: ${percent}%; background-color: ${barColor};"></div>
              </div>
            </div>

            <div class="cbs-meta-grid">
              <div class="cbs-meta-item">
                <span class="cbs-meta-label">DISTANCE REMAINING</span>
                <span class="cbs-meta-val">${kmText}</span>
              </div>
              <div class="cbs-meta-item">
                <span class="cbs-meta-label">DUE DATE</span>
                <span class="cbs-meta-val">${dateText}</span>
              </div>
            </div>

            <div class="cbs-card-footer">
              <span class="cbs-reset-count">Resets: ${item.reset_count}</span>
              <button class="btn-reset-cbs" data-type="${item.item_type}" data-name="${item.name}">
                RESET SERVICE
              </button>
            </div>
          </div>
        `;
      }).join('');

      // Attach reset button handlers
      document.querySelectorAll('.btn-reset-cbs').forEach(btn => {
        btn.addEventListener('click', (e) => {
          e.preventDefault();
          currentCbsResetItem = {
            type: btn.dataset.type,
            name: btn.dataset.name,
          };
          if (cbsModalItemText) {
            cbsModalItemText.innerText = `You are about to reset the service interval for "${currentCbsResetItem.name}" back to 100%.`;
          }
          if (cbsModal) cbsModal.style.display = 'flex';
        });
      });
    } catch (err) {
      cbsItemsGrid.innerHTML = `
        <div class="cbs-card" style="grid-column: 1 / -1; text-align: center; color: var(--text-muted);">
          Failed to load CBS report: ${err}
        </div>
      `;
    }
  }

  if (btnRefreshCbs) {
    btnRefreshCbs.addEventListener('click', () => {
      fetchCbsReport();
    });
  }

  if (btnCancelCbsReset) {
    btnCancelCbsReset.addEventListener('click', () => {
      if (cbsModal) cbsModal.style.display = 'none';
      currentCbsResetItem = null;
    });
  }

  if (btnConfirmCbsReset) {
    btnConfirmCbsReset.addEventListener('click', async () => {
      if (!currentCbsResetItem) return;
      try {
        const msg = await callBackend('reset_cbs', {
          itemType: currentCbsResetItem.type,
        });
        if (cbsModal) cbsModal.style.display = 'none';
        currentCbsResetItem = null;
        await fetchCbsReport();
        alert(msg || 'CBS service reset successfully executed!');
      } catch (err) {
        if (cbsModal) cbsModal.style.display = 'none';
        currentCbsResetItem = null;
        alert('Failed to reset CBS item: ' + err);
      }
    });
  }

  // ----------------------------------------------------
  // Topology Scan (ISTA-Style ECU Network Map)
  // ----------------------------------------------------
  let lastTopologyReport = null;

  async function fetchTopologyScan() {
    if (!topologyContainer) return;
    if (topoScanLoading) topoScanLoading.style.display = 'flex';
    if (btnScanTopology) btnScanTopology.disabled = true;

    try {
      const report = await callBackend('scan_topology');
      if (topoScanLoading) topoScanLoading.style.display = 'none';
      if (btnScanTopology) btnScanTopology.disabled = false;

      if (!report || !report.supported) {
        if (topologyUnsupportedBanner) topologyUnsupportedBanner.style.display = 'block';
        if (topoMetricsBar) topoMetricsBar.style.display = 'none';
        topologyContainer.innerHTML = '';
        return;
      }

      if (topologyUnsupportedBanner) topologyUnsupportedBanner.style.display = 'none';
      if (topoMetricsBar) topoMetricsBar.style.display = 'grid';
      lastTopologyReport = report;

      // Update metrics
      if (topoTotalScanned) topoTotalScanned.innerText = report.total_ecus_scanned;
      if (topoOnlineCount) topoOnlineCount.innerText = report.online_count;
      if (topoFaultCount) topoFaultCount.innerText = report.fault_count;
      if (topoHealthStatus) {
        if (report.fault_count === 0) {
          topoHealthStatus.innerText = 'ALL NORMAL';
          topoHealthStatus.className = 'topo-metric-val green';
        } else {
          topoHealthStatus.innerText = `${report.fault_count} FAULTS STORED`;
          topoHealthStatus.className = 'topo-metric-val amber';
        }
      }

      // Group nodes by bus type
      const busGroups = [
        { key: 'pt_can', name: 'PT-CAN (Powertrain Bus)', badgeClass: 'ptcan', label: '500 kbps · High-Speed CAN' },
        { key: 'flex_ray', name: 'FlexRay (Chassis & Dynamics Bus)', badgeClass: 'flexray', label: '10 Mbps · Deterministic Dual-Channel' },
        { key: 'k_can', name: 'K-CAN (Body & Interior Bus)', badgeClass: 'kcan', label: '100 kbps · Low-Speed Fault-Tolerant CAN' },
        { key: 'ethernet', name: 'Ethernet (Gateway & Multimedia)', badgeClass: 'ethernet', label: '100BASE-TX · 100 Mbps DoIP / BroadR-Reach' },
      ];

      topologyContainer.innerHTML = busGroups.map(group => {
        const nodes = report.nodes.filter(n => n.bus === group.key);
        if (!nodes || nodes.length === 0) return '';

        const nodesHtml = nodes.map(node => {
          let cardClass = 'normal';
          let pillClass = 'pill-green';
          let pillText = 'NORMAL';

          if (node.health === 'fault_stored') {
            cardClass = 'fault-stored';
            pillClass = 'pill-amber';
            pillText = `${node.dtc_count} FAULT${node.dtc_count > 1 ? 'S' : ''}`;
          } else if (node.health === 'comm_error') {
            cardClass = 'comm-error';
            pillClass = 'pill-red';
            pillText = 'COMM ERROR';
          } else if (node.health === 'no_response') {
            cardClass = 'no-response';
            pillClass = 'pill-grey';
            pillText = 'NO RESPONSE';
          }

          const hexAddr = '0x' + node.logical_address.toString(16).toUpperCase().padStart(4, '0');
          const latencyStr = node.response_time_ms != null ? `${node.response_time_ms} ms` : 'Timeout';

          return `
            <div class="ecu-card ${cardClass}" data-addr="${node.logical_address}">
              <div class="ecu-card-top">
                <span class="ecu-short-name">${node.short_name}</span>
                <span class="ecu-addr">${hexAddr}</span>
              </div>
              <div class="ecu-full-name">${node.full_name}</div>
              <div class="ecu-card-bottom">
                <span class="ecu-pill ${pillClass}">${pillText}</span>
                <span class="ecu-latency-text">${latencyStr}</span>
              </div>
            </div>
          `;
        }).join('');

        return `
          <div class="bus-section">
            <div class="bus-header">
              <div class="bus-title-group">
                <span class="bus-title">${group.name}</span>
                <span class="bus-badge ${group.badgeClass}">${nodes.length} CONTROLLERS</span>
              </div>
              <span style="font-size: 0.72rem; color: var(--text-dim);">${group.label}</span>
            </div>
            <div class="bus-nodes-grid">
              ${nodesHtml}
            </div>
          </div>
        `;
      }).join('');

      // Attach click listeners to cards
      document.querySelectorAll('.ecu-card').forEach(card => {
        card.addEventListener('click', () => {
          const addr = parseInt(card.dataset.addr, 10);
          const node = report.nodes.find(n => n.logical_address === addr);
          if (node) {
            openEcuInspector(node);
          }
        });
      });

    } catch (err) {
      if (topoScanLoading) topoScanLoading.style.display = 'none';
      if (btnScanTopology) btnScanTopology.disabled = false;
      if (topologyUnsupportedBanner) {
        topologyUnsupportedBanner.innerText = `Topology scan failed: ${err}`;
        topologyUnsupportedBanner.style.display = 'block';
      }
    }
  }

  function openEcuInspector(node) {
    if (!ecuNodeModal) return;
    const hexAddr = '0x' + node.logical_address.toString(16).toUpperCase().padStart(4, '0');
    if (ecuModalTitle) ecuModalTitle.innerText = `${node.short_name} (${hexAddr})`;
    if (ecuModalFullName) ecuModalFullName.innerText = node.full_name;

    if (ecuModalBadge) {
      if (node.health === 'normal') {
        ecuModalBadge.innerText = 'NORMAL';
        ecuModalBadge.className = 'ecu-status-badge pill-green';
      } else if (node.health === 'fault_stored') {
        ecuModalBadge.innerText = `${node.dtc_count} FAULT${node.dtc_count > 1 ? 'S' : ''}`;
        ecuModalBadge.className = 'ecu-status-badge pill-amber';
      } else if (node.health === 'comm_error') {
        ecuModalBadge.innerText = 'COMM ERROR';
        ecuModalBadge.className = 'ecu-status-badge pill-red';
      } else {
        ecuModalBadge.innerText = 'NO RESPONSE';
        ecuModalBadge.className = 'ecu-status-badge pill-grey';
      }
    }

    const busNames = {
      'pt_can': 'PT-CAN (Powertrain)',
      'flex_ray': 'FlexRay (Chassis & Dynamics)',
      'k_can': 'K-CAN (Body & Interior)',
      'ethernet': 'Ethernet (Gateway)',
    };

    if (ecuModalBus) ecuModalBus.innerText = busNames[node.bus] || node.bus;
    if (ecuModalAddress) ecuModalAddress.innerText = hexAddr;
    if (ecuModalLatency) ecuModalLatency.innerText = node.response_time_ms != null ? `${node.response_time_ms} ms` : 'Unreachable';
    if (ecuModalDtcCount) ecuModalDtcCount.innerText = node.dtc_count;

    if (ecuModalDtcList) {
      if (!node.dtcs || node.dtcs.length === 0) {
        ecuModalDtcList.innerHTML = `
          <div class="ecu-dtc-empty">
            ✓ No active diagnostic trouble codes stored in this ECU memory.
          </div>
        `;
      } else {
        ecuModalDtcList.innerHTML = node.dtcs.map(dtc => `
          <div class="ecu-dtc-row">
            <div class="ecu-dtc-code-group">
              <span class="ecu-dtc-code">${dtc.code}</span>
              <span class="ecu-dtc-desc">${dtc.description}</span>
            </div>
            <span class="dtc-status ${dtc.status.toLowerCase()}">${dtc.status.toUpperCase()}</span>
          </div>
        `).join('');
      }
    }

    ecuNodeModal.style.display = 'flex';
  }

  if (btnCloseEcuModal) {
    btnCloseEcuModal.addEventListener('click', () => {
      if (ecuNodeModal) ecuNodeModal.style.display = 'none';
    });
  }

  if (btnScanTopology) {
    btnScanTopology.addEventListener('click', () => {
      fetchTopologyScan();
    });
  }

  window.addEventListener('click', (e) => {
    if (e.target === ecuNodeModal) ecuNodeModal.style.display = 'none';
    if (e.target === waterPumpModal) waterPumpModal.style.display = 'none';
  });

  // ----------------------------------------------------
  // BMW Actuator Service Routines
  // ----------------------------------------------------
  let bleedPollInterval = null;

  async function fetchActuatorsStatus() {
    await fetchWaterPumpStatus();
    await fetchTransmissionStatus();
  }

  async function fetchWaterPumpStatus() {
    try {
      const status = await callBackend('get_water_pump_bleed_status');
      if (!status) return;

      if (bleedProgressFill) bleedProgressFill.style.width = `${status.progress_percent}%`;
      if (bleedProgressVal) bleedProgressVal.innerText = `${Math.round(status.progress_percent)}%`;
      if (bleedStageName) bleedStageName.innerText = status.stage_description;
      if (bleedPumpSpeed) bleedPumpSpeed.innerText = `${status.pump_speed_percent}% PWM`;
      if (bleedElapsed) bleedElapsed.innerText = `${status.elapsed_seconds}s`;

      const remMin = Math.floor(status.remaining_seconds / 60);
      const remSec = status.remaining_seconds % 60;
      if (bleedRemaining) bleedRemaining.innerText = `${remMin}m ${remSec.toString().padStart(2, '0')}s`;

      if (status.is_running) {
        if (btnStartBleed) btnStartBleed.style.display = 'none';
        if (btnStopBleed) btnStopBleed.style.display = 'inline-block';
        if (!bleedPollInterval) {
          bleedPollInterval = setInterval(fetchWaterPumpStatus, 1000);
        }
      } else {
        if (btnStartBleed) btnStartBleed.style.display = 'inline-block';
        if (btnStopBleed) btnStopBleed.style.display = 'none';
        if (bleedPollInterval) {
          clearInterval(bleedPollInterval);
          bleedPollInterval = null;
        }
      }
    } catch (_) {}
  }

  async function fetchTransmissionStatus() {
    try {
      const status = await callBackend('get_transmission_oil_status');
      if (!status || !status.supported) return;

      if (transTempC) transTempC.innerText = `${status.temperature_c.toFixed(1)} °C`;
      if (transTempF) transTempF.innerText = `(${status.temperature_f.toFixed(1)} °F)`;
      if (transStatusMsg) transStatusMsg.innerText = status.status_text;

      if (transWindowBadge) {
        if (status.temperature_c < 40.0) {
          transWindowBadge.innerText = 'COLD (<40°C)';
          transWindowBadge.className = 'trans-window-badge cold';
        } else if (status.temperature_c <= 50.0) {
          transWindowBadge.innerText = 'OPTIMAL FILL WINDOW (40-50°C)';
          transWindowBadge.className = 'trans-window-badge target';
        } else {
          transWindowBadge.innerText = 'TOO HOT (>50°C)';
          transWindowBadge.className = 'trans-window-badge hot';
        }
      }
    } catch (_) {}
  }

  if (btnStartBleed) {
    btnStartBleed.addEventListener('click', () => {
      if (waterPumpModal) waterPumpModal.style.display = 'flex';
    });
  }

  if (btnCancelBleed) {
    btnCancelBleed.addEventListener('click', () => {
      if (waterPumpModal) waterPumpModal.style.display = 'none';
    });
  }

  if (btnConfirmBleed) {
    btnConfirmBleed.addEventListener('click', async () => {
      if (waterPumpModal) waterPumpModal.style.display = 'none';
      try {
        const msg = await callBackend('start_water_pump_bleed');
        if (bleedStatusText) bleedStatusText.innerText = msg;
        await fetchWaterPumpStatus();
      } catch (err) {
        alert('Failed to start water pump bleed routine: ' + err);
      }
    });
  }

  if (btnStopBleed) {
    btnStopBleed.addEventListener('click', async () => {
      try {
        const msg = await callBackend('stop_water_pump_bleed');
        if (bleedStatusText) bleedStatusText.innerText = msg;
        await fetchWaterPumpStatus();
      } catch (err) {
        alert('Failed to stop water pump bleed: ' + err);
      }
    });
  }

  if (btnStartTransFill) {
    btnStartTransFill.addEventListener('click', async () => {
      try {
        const msg = await callBackend('start_transmission_fill_routine');
        if (transRoutineStatus) transRoutineStatus.innerText = msg;
        await fetchTransmissionStatus();
      } catch (err) {
        alert('Failed to execute transmission level routine: ' + err);
      }
    });
  }

  btnFanSteps.forEach(btn => {
    btn.addEventListener('click', async () => {
      const duty = parseInt(btn.dataset.duty, 10);
      try {
        const msg = await callBackend('set_radiator_fan_duty', { dutyPercent: duty });
        if (fanDutyVal) {
          fanDutyVal.innerText = duty === 0 ? 'AUTO (DME CONTROL)' : `FORCED ${duty}% PWM`;
        }
        if (fanStatusText) fanStatusText.innerText = msg;
      } catch (err) {
        alert('Failed to command fan speed: ' + err);
      }
    });
  });

  if (btnOpenFlap) {
    btnOpenFlap.addEventListener('click', async () => {
      try {
        const msg = await callBackend('set_exhaust_flap', { open: true });
        if (flapStateVal) flapStateVal.innerText = 'OPEN (SPORT SOUND)';
        if (flapStatusText) flapStatusText.innerText = msg;
      } catch (err) {
        alert('Failed to actuate exhaust flap: ' + err);
      }
    });
  }

  if (btnCloseFlap) {
    btnCloseFlap.addEventListener('click', async () => {
      try {
        const msg = await callBackend('set_exhaust_flap', { open: false });
        if (flapStateVal) flapStateVal.innerText = 'CLOSED (QUIET MODE)';
        if (flapStatusText) flapStatusText.innerText = msg;
      } catch (err) {
        alert('Failed to actuate exhaust flap: ' + err);
      }
    });
  }

  if (btnRefreshActuators) {
    btnRefreshActuators.addEventListener('click', () => {
      fetchActuatorsStatus();
    });
  }

  // Documentation Sidebar Navigation
  const docLinks = document.querySelectorAll('.doc-link');
  const docArticles = document.querySelectorAll('.doc-article');
  docLinks.forEach(link => {
    link.addEventListener('click', () => {
      const docId = link.dataset.doc;
      if (!docId) return;
      docLinks.forEach(l => l.classList.remove('active'));
      docArticles.forEach(a => a.classList.remove('active'));
      link.classList.add('active');
      const targetArticle = document.getElementById(`doc-${docId}`);
      if (targetArticle) {
        targetArticle.classList.add('active');
      }
    });
  });

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

// obd2mon - Vehicle Diagnostic Dashboard Client Controller

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
  console.log('[obd2mon] Initializing dashboard UI controller...');

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
      if (targetPage === 'profiles') {
        fetchGarage();
      }
      if (targetPage === 'coding') {
        fetchCodingParameters();
      }
      if (targetPage === 'datalog') {
        startDatalogStream();
      } else {
        stopDatalogStream();
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

  // ====================================================
  // GARAGE & VEHICLE PROFILES CONTROLLER
  // ====================================================
  let currentGarage = null;
  let viewingVehicleId = null;
  let currentHistoryData = null;
  let currentHistoryFilter = 'all';

  // DOM elements for Garage
  const btnDetectVin = document.getElementById('btnDetectVin');
  const btnAddVehicle = document.getElementById('btnAddVehicle');
  const heroVehicleTitle = document.getElementById('heroVehicleTitle');
  const heroVehicleNotes = document.getElementById('heroVehicleNotes');
  const btnViewActiveHistory = document.getElementById('btnViewActiveHistory');
  const btnEditActiveVehicle = document.getElementById('btnEditActiveVehicle');
  const heroVin = document.getElementById('heroVin');
  const heroMileage = document.getElementById('heroMileage');
  const heroEngine = document.getElementById('heroEngine');
  const heroInterface = document.getElementById('heroInterface');
  const garageCountPill = document.getElementById('garageCountPill');
  const garageVehiclesGrid = document.getElementById('garageVehiclesGrid');

  // Editor Modal elements
  const vehicleEditorModal = document.getElementById('vehicleEditorModal');
  const vehEditorTitle = document.getElementById('vehEditorTitle');
  const btnCancelVehEditorX = document.getElementById('btnCancelVehEditorX');
  const btnCancelVehEditor = document.getElementById('btnCancelVehEditor');
  const vehicleEditForm = document.getElementById('vehicleEditForm');
  const vehEditId = document.getElementById('vehEditId');
  const vehEditNickname = document.getElementById('vehEditNickname');
  const vehEditYear = document.getElementById('vehEditYear');
  const vehEditMake = document.getElementById('vehEditMake');
  const vehEditModel = document.getElementById('vehEditModel');
  const vehEditEngine = document.getElementById('vehEditEngine');
  const vehEditVin = document.getElementById('vehEditVin');
  const btnAutoQueryVin = document.getElementById('btnAutoQueryVin');
  const vehEditMileage = document.getElementById('vehEditMileage');
  const vehEditMileageUnit = document.getElementById('vehEditMileageUnit');
  const vehEditInterface = document.getElementById('vehEditInterface');
  const enetFields = document.getElementById('enetFields');
  const vehEditEnetHost = document.getElementById('vehEditEnetHost');
  const vehEditEnetPort = document.getElementById('vehEditEnetPort');
  const serialFields = document.getElementById('serialFields');
  const vehEditSerialPort = document.getElementById('vehEditSerialPort');
  const vehEditSerialBaud = document.getElementById('vehEditSerialBaud');
  const vehEditNotes = document.getElementById('vehEditNotes');
  const compatRecTitle = document.getElementById('compatRecTitle');
  const compatSupportedList = document.getElementById('compatSupportedList');
  const compatUnsupportedList = document.getElementById('compatUnsupportedList');
  const compatUnsupportedCol = document.getElementById('compatUnsupportedCol');
  const compatCableAdvice = document.getElementById('compatCableAdvice');
  const btnSaveVehProfileOnly = document.getElementById('btnSaveVehProfileOnly');

  // History Modal elements
  const vehicleHistoryModal = document.getElementById('vehicleHistoryModal');
  const histModalTitle = document.getElementById('histModalTitle');
  const histModalSubtitle = document.getElementById('histModalSubtitle');
  const btnCloseHistoryModalX = document.getElementById('btnCloseHistoryModalX');
  const btnCloseHistoryModal = document.getElementById('btnCloseHistoryModal');
  const histCountAll = document.getElementById('histCountAll');
  const histCountService = document.getElementById('histCountService');
  const histCountDtc = document.getElementById('histCountDtc');
  const histCountDatalog = document.getElementById('histCountDatalog');
  const toggleAddLogHeader = document.getElementById('toggleAddLogHeader');
  const addLogToggleIcon = document.getElementById('addLogToggleIcon');
  const addLogBody = document.getElementById('addLogBody');
  const manualLogCategory = document.getElementById('manualLogCategory');
  const manualLogMileage = document.getElementById('manualLogMileage');
  const manualLogText = document.getElementById('manualLogText');
  const btnSaveManualLog = document.getElementById('btnSaveManualLog');
  const historyTimeline = document.getElementById('historyTimeline');

  async function fetchGarage() {
    try {
      const garage = await callBackend('get_garage');
      if (garage) {
        currentGarage = garage;
        renderGarage();
      }
    } catch (err) {
      console.error('Failed to load garage:', err);
    }
  }

  function renderGarage() {
    if (!currentGarage) return;
    const activeVeh = currentGarage.vehicles.find(v => v.id === currentGarage.active_vehicle_id) || currentGarage.vehicles[0];

    if (activeVeh) {
      if (heroVehicleTitle) heroVehicleTitle.innerText = activeVeh.nickname || `${activeVeh.year} ${activeVeh.make} ${activeVeh.model}`;
      if (heroVehicleNotes) heroVehicleNotes.innerText = activeVeh.notes || 'No vehicle notes specified.';
      if (heroVin) heroVin.innerText = activeVeh.vin || 'Not Entered';
      if (heroMileage) {
        const miVal = activeVeh.mileage != null ? activeVeh.mileage.toLocaleString() : '--';
        heroMileage.innerText = `${miVal} ${activeVeh.mileage_unit || 'km'}`;
      }
      if (heroEngine) heroEngine.innerText = activeVeh.engine || 'Standard Powertrain';
      if (heroInterface) {
        if (activeVeh.interface_type === 'enet') {
          heroInterface.innerText = `BMW ENET (${activeVeh.interface_host || '169.254.1.1'}:${activeVeh.interface_port || 6801})`;
        } else if (activeVeh.interface_type === 'bluetooth' || activeVeh.interface_type === 'usb') {
          heroInterface.innerText = `Serial ${activeVeh.interface_type.toUpperCase()} (${activeVeh.serial_port || 'COM3'} @ ${activeVeh.serial_baud || 38400})`;
        } else {
          heroInterface.innerText = 'Diagnostic Simulator (Mock Engine)';
        }
      }

      // Update header banner vehicle name
      const vName = document.getElementById('vehicleName');
      if (vName) {
        vName.innerText = activeVeh.nickname || `${activeVeh.year} ${activeVeh.make} ${activeVeh.model}`;
      }
    }

    if (garageCountPill) {
      garageCountPill.innerText = `${currentGarage.vehicles.length} Vehicle${currentGarage.vehicles.length === 1 ? '' : 's'}`;
    }

    if (garageVehiclesGrid) {
      garageVehiclesGrid.innerHTML = currentGarage.vehicles.map(veh => {
        const isActive = veh.id === currentGarage.active_vehicle_id;
        let ifaceDesc = '';
        if (veh.interface_type === 'enet') {
          ifaceDesc = `ENET (${veh.interface_host || '169.254.1.1'}:${veh.interface_port || 6801})`;
        } else if (veh.interface_type === 'bluetooth' || veh.interface_type === 'usb') {
          ifaceDesc = `${veh.interface_type.toUpperCase()} (${veh.serial_port || 'COM3'} @ ${veh.serial_baud || 38400})`;
        } else {
          ifaceDesc = 'Diagnostic Simulator';
        }

        const mileageStr = veh.mileage != null ? `${veh.mileage.toLocaleString()} ${veh.mileage_unit || 'km'}` : 'Not specified';
        const vinStr = veh.vin || 'Not recorded';

        return `
          <div class="vehicle-card ${isActive ? 'is-active' : ''}" data-vehicle-id="${veh.id}">
            <div>
              <div class="vcard-header">
                <div class="vcard-title">
                  <h4>${veh.nickname}</h4>
                  <span class="vcard-make-model">${veh.year} ${veh.make} ${veh.model}</span>
                </div>
                ${isActive ? '<span class="vcard-active-pill">ACTIVE</span>' : ''}
              </div>

              <div class="vcard-meta-list">
                <div class="vcard-meta-row"><span class="lbl">VIN:</span><span class="val">${vinStr}</span></div>
                <div class="vcard-meta-row"><span class="lbl">Odometer:</span><span class="val">${mileageStr}</span></div>
                <div class="vcard-meta-row"><span class="lbl">Interface:</span><span class="val">${ifaceDesc}</span></div>
                <div class="vcard-meta-row"><span class="lbl">Engine:</span><span class="val">${veh.engine || '--'}</span></div>
              </div>

              ${veh.notes ? `<div class="vcard-notes">📝 ${veh.notes}</div>` : ''}
            </div>

            <div class="vcard-actions">
              ${!isActive ? `<button class="btn btn-primary btn-switch-veh" data-vehicle-id="${veh.id}">SWITCH & CONNECT</button>` : `<button class="btn btn-secondary" disabled>ACTIVE PROFILE</button>`}
              <button class="btn btn-secondary btn-history-veh" data-vehicle-id="${veh.id}">VIEW LOGS</button>
              <button class="btn btn-outline btn-edit-veh" data-vehicle-id="${veh.id}">EDIT</button>
              ${currentGarage.vehicles.length > 1 ? `<button class="btn btn-danger btn-delete-veh" data-vehicle-id="${veh.id}">DELETE</button>` : ''}
            </div>
          </div>
        `;
      }).join('');

      garageVehiclesGrid.querySelectorAll('.btn-switch-veh').forEach(btn => {
        btn.addEventListener('click', () => switchActiveVehicle(btn.dataset.vehicleId));
      });
      garageVehiclesGrid.querySelectorAll('.btn-history-veh').forEach(btn => {
        btn.addEventListener('click', () => openVehicleHistory(btn.dataset.vehicleId));
      });
      garageVehiclesGrid.querySelectorAll('.btn-edit-veh').forEach(btn => {
        btn.addEventListener('click', () => openVehicleEditor(btn.dataset.vehicleId));
      });
      garageVehiclesGrid.querySelectorAll('.btn-delete-veh').forEach(btn => {
        btn.addEventListener('click', () => confirmDeleteVehicle(btn.dataset.vehicleId));
      });
    }
  }

  async function switchActiveVehicle(vehicleId) {
    try {
      const garage = await callBackend('set_active_vehicle', { vehicleId });
      if (garage) {
        currentGarage = garage;
        renderGarage();
        const dashBtn = document.querySelector('.nav-btn[data-page="dashboard"]');
        if (dashBtn) dashBtn.click();
      }
    } catch (err) {
      alert('Failed to activate vehicle: ' + err);
    }
  }

  function updateInterfaceFields() {
    if (!vehEditInterface) return;
    const iface = vehEditInterface.value;
    if (iface === 'enet') {
      if (enetFields) enetFields.style.display = 'flex';
      if (serialFields) serialFields.style.display = 'none';
    } else if (iface === 'bluetooth' || iface === 'usb') {
      if (enetFields) enetFields.style.display = 'none';
      if (serialFields) serialFields.style.display = 'flex';
    } else {
      if (enetFields) enetFields.style.display = 'none';
      if (serialFields) serialFields.style.display = 'none';
    }
    evaluateFormCompatibility();
  }

  async function evaluateFormCompatibility() {
    if (!vehEditMake || !vehEditModel) return;
    const make = vehEditMake.value || 'General / Generic';
    const model = vehEditModel.value || 'Standard';
    const year = parseInt(vehEditYear.value, 10) || 2015;
    const interfaceType = vehEditInterface.value || 'bluetooth';

    try {
      const compat = await callBackend('get_vehicle_compatibility', {
        make,
        model,
        year,
        interfaceType,
      });

      if (compat) {
        if (compatRecTitle) compatRecTitle.innerText = `Recommended: ${compat.recommended_interface}`;

        if (compatSupportedList) {
          compatSupportedList.innerHTML = compat.supported_features.length > 0
            ? compat.supported_features.map(f => `<li>✓ ${f}</li>`).join('')
            : '<li>No features available on this protocol combination</li>';
        }

        if (compatUnsupportedList) {
          if (compat.unsupported_features.length > 0) {
            compatUnsupportedList.innerHTML = compat.unsupported_features.map(f => `<li>⚠️ ${f}</li>`).join('');
            if (compatUnsupportedCol) compatUnsupportedCol.style.display = 'block';
          } else {
            compatUnsupportedList.innerHTML = '<li>✓ All features fully supported</li>';
            if (compatUnsupportedCol) compatUnsupportedCol.style.display = 'none';
          }
        }

        if (compatCableAdvice) {
          compatCableAdvice.innerText = compat.cable_notes;
        }
      }
    } catch (err) {
      console.warn('Compatibility evaluation error:', err);
    }
  }

  function applyPreset(preset) {
    if (preset === 'blank') {
      vehEditId.value = '';
      vehEditNickname.value = '';
      vehEditYear.value = new Date().getFullYear();
      vehEditMake.value = '';
      vehEditModel.value = '';
      vehEditEngine.value = '';
      vehEditVin.value = '';
      vehEditMileage.value = '';
      vehEditInterface.value = 'bluetooth';
      vehEditNotes.value = '';
    } else if (preset === 'generic') {
      vehEditNickname.value = 'Universal OBD-II Vehicle';
      vehEditYear.value = 2016;
      vehEditMake.value = 'General / Generic';
      vehEditModel.value = 'SAE J1979 Standard';
      vehEditEngine.value = 'Standard Engine';
      vehEditInterface.value = 'bluetooth';
      vehEditSerialPort.value = 'COM3';
      vehEditSerialBaud.value = '38400';
      vehEditNotes.value = 'Universal emissions monitoring on standard SAE J1979 Mode 01-09.';
    } else if (preset === 'bmw_f10') {
      vehEditNickname.value = '2011 BMW 535i xDrive (F10)';
      vehEditYear.value = 2011;
      vehEditMake.value = 'BMW';
      vehEditModel.value = '535i xDrive (F10)';
      vehEditEngine.value = '3.0L Turbo Inline-6 (N55B30M0)';
      vehEditInterface.value = 'enet';
      vehEditEnetHost.value = '169.254.1.1';
      vehEditEnetPort.value = 6801;
      vehEditNotes.value = 'Full ISTA routines: Battery registration, CBS reset, water pump bleed, N55 datalogger.';
    } else if (preset === 'bmw_e90') {
      vehEditNickname.value = '2008 BMW 335i (E90)';
      vehEditYear.value = 2008;
      vehEditMake.value = 'BMW';
      vehEditModel.value = '335i Sedan (E90)';
      vehEditEngine.value = '3.0L Twin-Turbo (N54B30)';
      vehEditInterface.value = 'usb';
      vehEditSerialPort.value = 'COM1';
      vehEditSerialBaud.value = '38400';
      vehEditNotes.value = 'Connected using K+DCAN USB cable with pin 7-8 bridge.';
    } else if (preset === 'gmc') {
      vehEditNickname.value = '2010 GMC Acadia SLE';
      vehEditYear.value = 2010;
      vehEditMake.value = 'GMC';
      vehEditModel.value = 'Acadia SLE';
      vehEditEngine.value = '3.6L V6 Direct Injection (LLT)';
      vehEditInterface.value = 'bluetooth';
      vehEditSerialPort.value = 'COM3';
      vehEditSerialBaud.value = '38400';
      vehEditNotes.value = 'Connected via ELM327 Bluetooth. Uses GM Mode 22 enhanced PIDs.';
    }

    updateInterfaceFields();
  }

  function openVehicleEditor(vehicleId) {
    if (!vehicleEditorModal) return;

    if (!vehicleId) {
      if (vehEditorTitle) vehEditorTitle.innerText = 'ADD NEW VEHICLE TO GARAGE';
      applyPreset('blank');
    } else {
      const veh = currentGarage && currentGarage.vehicles.find(v => v.id === vehicleId);
      if (!veh) return;
      if (vehEditorTitle) vehEditorTitle.innerText = `EDIT VEHICLE: ${veh.nickname}`;
      vehEditId.value = veh.id;
      vehEditNickname.value = veh.nickname;
      vehEditYear.value = veh.year;
      vehEditMake.value = veh.make;
      vehEditModel.value = veh.model;
      vehEditEngine.value = veh.engine || '';
      vehEditVin.value = veh.vin || '';
      vehEditMileage.value = veh.mileage != null ? veh.mileage : '';
      vehEditMileageUnit.value = veh.mileage_unit || 'km';
      vehEditInterface.value = veh.interface_type || 'enet';
      if (veh.interface_host) vehEditEnetHost.value = veh.interface_host;
      if (veh.interface_port) vehEditEnetPort.value = veh.interface_port;
      if (veh.serial_port) vehEditSerialPort.value = veh.serial_port;
      if (veh.serial_baud) vehEditSerialBaud.value = veh.serial_baud;
      vehEditNotes.value = veh.notes || '';
      updateInterfaceFields();
    }

    vehicleEditorModal.style.display = 'flex';
  }

  async function saveVehicleFromForm(setAsActive) {
    const nickname = vehEditNickname.value.trim();
    const make = vehEditMake.value.trim();
    const model = vehEditModel.value.trim();
    const year = parseInt(vehEditYear.value, 10);

    if (!nickname || !make || !model || isNaN(year)) {
      alert('Please fill in profile nickname, year, make, and model.');
      return;
    }

    const mileageNum = parseInt(vehEditMileage.value, 10);
    const vehicleRecord = {
      id: vehEditId.value.trim(),
      nickname,
      make,
      model,
      year,
      vin: vehEditVin.value.trim().toUpperCase() || null,
      mileage: !isNaN(mileageNum) ? mileageNum : null,
      mileage_unit: vehEditMileageUnit.value,
      engine: vehEditEngine.value.trim() || null,
      interface_type: vehEditInterface.value,
      interface_host: vehEditInterface.value === 'enet' ? vehEditEnetHost.value.trim() : null,
      interface_port: vehEditInterface.value === 'enet' ? parseInt(vehEditEnetPort.value, 10) : null,
      serial_port: (vehEditInterface.value === 'bluetooth' || vehEditInterface.value === 'usb') ? vehEditSerialPort.value.trim() : null,
      serial_baud: (vehEditInterface.value === 'bluetooth' || vehEditInterface.value === 'usb') ? parseInt(vehEditSerialBaud.value, 10) : null,
      notes: vehEditNotes.value.trim() || null,
      created_at: '',
      last_connected_at: null,
      active_dtc_count: 0,
    };

    try {
      const garage = await callBackend('save_vehicle_profile', {
        vehicle: vehicleRecord,
        setAsActive,
      });
      if (garage) {
        currentGarage = garage;
        renderGarage();
        if (vehicleEditorModal) vehicleEditorModal.style.display = 'none';
        if (setAsActive) {
          const dashBtn = document.querySelector('.nav-btn[data-page="dashboard"]');
          if (dashBtn) dashBtn.click();
        }
      }
    } catch (err) {
      alert('Failed to save vehicle profile: ' + err);
    }
  }

  async function confirmDeleteVehicle(vehicleId) {
    const veh = currentGarage && currentGarage.vehicles.find(v => v.id === vehicleId);
    const name = veh ? veh.nickname : 'this vehicle';
    if (!confirm(`Are you sure you want to delete "${name}" from your garage? Historical logs will be preserved on disk.`)) {
      return;
    }

    try {
      const garage = await callBackend('delete_vehicle_profile', { vehicleId });
      if (garage) {
        currentGarage = garage;
        renderGarage();
      }
    } catch (err) {
      alert('Failed to delete vehicle profile: ' + err);
    }
  }

  async function detectVinFromCar() {
    try {
      const res = await callBackend('detect_vehicle_vin');
      if (res && res.vin) {
        if (res.matched_vehicle_id) {
          const veh = currentGarage && currentGarage.vehicles.find(v => v.id === res.matched_vehicle_id);
          const name = veh ? veh.nickname : res.matched_vehicle_id;
          if (confirm(`Detected VIN: ${res.vin}\nMatches saved vehicle "${name}".\n\nWould you like to switch to this profile now?`)) {
            switchActiveVehicle(res.matched_vehicle_id);
          }
        } else {
          if (confirm(`Detected VIN: ${res.vin}\n\nNo saved profile matches this VIN. Would you like to create a new vehicle profile with this VIN?`)) {
            openVehicleEditor(null);
            if (vehEditVin) vehEditVin.value = res.vin;
          }
        }
      }
    } catch (err) {
      alert('Unable to read VIN from vehicle bus. Verify connection and ignition ON: ' + err);
    }
  }

  async function openVehicleHistory(vehicleId) {
    if (!vehicleHistoryModal) return;
    viewingVehicleId = vehicleId;
    const veh = currentGarage && currentGarage.vehicles.find(v => v.id === vehicleId);
    if (!veh) return;

    if (histModalTitle) histModalTitle.innerText = `LOGBOOK & HISTORY: ${veh.nickname}`;
    const miStr = veh.mileage != null ? `${veh.mileage.toLocaleString()} ${veh.mileage_unit || 'km'}` : 'Mileage not set';
    if (histModalSubtitle) histModalSubtitle.innerText = `${veh.year} ${veh.make} ${veh.model} · VIN: ${veh.vin || 'None'} · ${miStr}`;

    try {
      const history = await callBackend('get_vehicle_history', { vehicleId });
      currentHistoryData = history || { service_logs: [], dtc_history: [], datalog_logs: [] };
      renderHistoryItems();
      vehicleHistoryModal.style.display = 'flex';
    } catch (err) {
      alert('Failed to load vehicle history: ' + err);
    }
  }

  function renderHistoryItems() {
    if (!currentHistoryData || !historyTimeline) return;

    const sLogs = currentHistoryData.service_logs || [];
    const dHistory = currentHistoryData.dtc_history || [];
    const dlLogs = currentHistoryData.datalog_logs || [];

    if (histCountAll) histCountAll.innerText = sLogs.length + dHistory.length + dlLogs.length;
    if (histCountService) histCountService.innerText = sLogs.length;
    if (histCountDtc) histCountDtc.innerText = dHistory.length;
    if (histCountDatalog) histCountDatalog.innerText = dlLogs.length;

    let items = [];

    if (currentHistoryFilter === 'all' || currentHistoryFilter === 'service') {
      sLogs.forEach(entry => {
        items.push({
          type: 'service',
          category: entry.category,
          date: entry.timestamp,
          mileage: entry.mileage,
          desc: entry.description,
          result: entry.result,
        });
      });
    }

    if (currentHistoryFilter === 'all' || currentHistoryFilter === 'dtc') {
      dHistory.forEach(entry => {
        const codesStr = entry.dtcs.map(d => d.code).join(', ');
        items.push({
          type: 'dtc',
          category: 'dtc_scan',
          date: entry.timestamp,
          mileage: entry.mileage,
          desc: `Scanned ${entry.dtc_count} DTC fault codes: ${codesStr || 'No faults found'}`,
          result: entry.dtc_count === 0 ? 'CLEAN' : `${entry.dtc_count} FAULTS`,
        });
      });
    }

    if (currentHistoryFilter === 'all' || currentHistoryFilter === 'datalog') {
      dlLogs.forEach(entry => {
        items.push({
          type: 'datalog',
          category: 'datalog',
          date: entry.timestamp,
          desc: `High-Speed Datalog: ${entry.sample_count} samples (${entry.duration_seconds.toFixed(1)}s) exported to ${entry.filename}`,
          result: `${(entry.file_size_bytes / 1024).toFixed(1)} KB`,
        });
      });
    }

    items.sort((a, b) => (b.date || '').localeCompare(a.date || ''));

    if (items.length === 0) {
      historyTimeline.innerHTML = `
        <div class="history-empty-state">
          No records logged under this vehicle profile yet.<br>
          Any battery registrations, CBS resets, actuator tests, DTC scans, and datalog recordings will automatically appear here.
        </div>
      `;
      return;
    }

    historyTimeline.innerHTML = items.map(item => {
      const timeStr = item.date ? item.date.replace('T', ' ').substring(0, 19) : 'Recorded';
      const miBadge = item.mileage ? ` · ${item.mileage.toLocaleString()} mi/km` : '';
      return `
        <div class="history-item-row">
          <div class="history-item-left">
            <div class="history-item-top">
              <span class="hist-badge ${item.category}">${item.category.replace('_', ' ')}</span>
              <span class="hist-time">${timeStr}${miBadge}</span>
            </div>
            <span class="hist-desc">${item.desc}</span>
          </div>
          <span class="hist-result">${item.result}</span>
        </div>
      `;
    }).join('');
  }

  // Wire Garage Event Listeners
  if (btnDetectVin) {
    btnDetectVin.addEventListener('click', detectVinFromCar);
  }
  if (btnAddVehicle) {
    btnAddVehicle.addEventListener('click', () => openVehicleEditor(null));
  }
  if (btnEditActiveVehicle) {
    btnEditActiveVehicle.addEventListener('click', () => {
      if (currentGarage) openVehicleEditor(currentGarage.active_vehicle_id);
    });
  }
  if (btnViewActiveHistory) {
    btnViewActiveHistory.addEventListener('click', () => {
      if (currentGarage) openVehicleHistory(currentGarage.active_vehicle_id);
    });
  }

  if (btnCancelVehEditor) {
    btnCancelVehEditor.addEventListener('click', () => {
      if (vehicleEditorModal) vehicleEditorModal.style.display = 'none';
    });
  }
  if (btnCancelVehEditorX) {
    btnCancelVehEditorX.addEventListener('click', () => {
      if (vehicleEditorModal) vehicleEditorModal.style.display = 'none';
    });
  }

  if (vehEditInterface) {
    vehEditInterface.addEventListener('change', updateInterfaceFields);
  }
  if (vehEditMake) {
    vehEditMake.addEventListener('input', evaluateFormCompatibility);
  }
  if (vehEditModel) {
    vehEditModel.addEventListener('input', evaluateFormCompatibility);
  }
  if (vehEditYear) {
    vehEditYear.addEventListener('input', evaluateFormCompatibility);
  }

  document.querySelectorAll('.preset-btn').forEach(btn => {
    btn.addEventListener('click', () => applyPreset(btn.dataset.preset));
  });

  if (btnAutoQueryVin) {
    btnAutoQueryVin.addEventListener('click', async () => {
      try {
        const res = await callBackend('detect_vehicle_vin');
        if (res && res.vin) {
          if (vehEditVin) vehEditVin.value = res.vin;
          alert('Successfully read VIN from vehicle bus: ' + res.vin);
        }
      } catch (err) {
        alert('Could not read VIN from bus: ' + err);
      }
    });
  }

  if (vehicleEditForm) {
    vehicleEditForm.addEventListener('submit', (e) => {
      e.preventDefault();
      saveVehicleFromForm(true);
    });
  }
  if (btnSaveVehProfileOnly) {
    btnSaveVehProfileOnly.addEventListener('click', () => {
      saveVehicleFromForm(false);
    });
  }

  if (btnCloseHistoryModal) {
    btnCloseHistoryModal.addEventListener('click', () => {
      if (vehicleHistoryModal) vehicleHistoryModal.style.display = 'none';
    });
  }
  if (btnCloseHistoryModalX) {
    btnCloseHistoryModalX.addEventListener('click', () => {
      if (vehicleHistoryModal) vehicleHistoryModal.style.display = 'none';
    });
  }

  document.querySelectorAll('.hist-tab').forEach(tab => {
    tab.addEventListener('click', () => {
      document.querySelectorAll('.hist-tab').forEach(t => t.classList.remove('active'));
      tab.classList.add('active');
      currentHistoryFilter = tab.dataset.filter || 'all';
      renderHistoryItems();
    });
  });

  if (toggleAddLogHeader) {
    toggleAddLogHeader.addEventListener('click', () => {
      if (!addLogBody) return;
      const isOpen = addLogBody.style.display !== 'none';
      addLogBody.style.display = isOpen ? 'none' : 'block';
      if (addLogToggleIcon) addLogToggleIcon.innerText = isOpen ? '▼' : '▲';
    });
  }

  if (btnSaveManualLog) {
    btnSaveManualLog.addEventListener('click', async () => {
      if (!viewingVehicleId) return;
      const text = manualLogText.value.trim();
      if (!text) {
        alert('Please enter a description for the service record.');
        return;
      }
      const category = manualLogCategory.value;
      const mileage = parseInt(manualLogMileage.value, 10);
      try {
        const updatedHistory = await callBackend('add_vehicle_service_note', {
          vehicleId: viewingVehicleId,
          category,
          note: text,
          mileage: !isNaN(mileage) ? mileage : null,
        });
        currentHistoryData = updatedHistory;
        manualLogText.value = '';
        renderHistoryItems();
        alert('Service record added to vehicle logbook!');
      } catch (err) {
        alert('Failed to save service note: ' + err);
      }
    });
  }

  window.addEventListener('click', (e) => {
    if (e.target === vehicleEditorModal) vehicleEditorModal.style.display = 'none';
    if (e.target === vehicleHistoryModal) vehicleHistoryModal.style.display = 'none';
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
      if (typeof bat.voltage === 'number') {
        updateCodingVoltage(bat.voltage);
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

  // Datalogger References
  const btnStartLog = document.getElementById('btnStartLog');
  const btnStopLog = document.getElementById('btnStopLog');
  const btnDownloadCsv = document.getElementById('btnDownloadCsv');
  const logRecordPill = document.getElementById('logRecordPill');
  const datalogUnsupportedBanner = document.getElementById('datalogUnsupportedBanner');
  const datalogCanvas = document.getElementById('datalogCanvas');

  // Datalogger Legend & Readouts
  const legendTargetBoost = document.getElementById('legendTargetBoost');
  const legendActualBoost = document.getElementById('legendActualBoost');
  const legendWgdc = document.getElementById('legendWgdc');
  const legendRailPress = document.getElementById('legendRailPress');
  const legendRpm = document.getElementById('legendRpm');

  const valTargetBoost = document.getElementById('valTargetBoost');
  const valTargetBoostHpa = document.getElementById('valTargetBoostHpa');
  const valActualBoost = document.getElementById('valActualBoost');
  const valActualBoostHpa = document.getElementById('valActualBoostHpa');
  const boostDevBadge = document.getElementById('boostDevBadge');
  const valWgdcText = document.getElementById('valWgdcText');
  const wgdcBarFill = document.getElementById('wgdcBarFill');

  const valRailTarget = document.getElementById('valRailTarget');
  const valRailTargetPsi = document.getElementById('valRailTargetPsi');
  const valRailActual = document.getElementById('valRailActual');
  const valRailActualPsi = document.getElementById('valRailActualPsi');
  const fuelHealthBadge = document.getElementById('fuelHealthBadge');
  const valLpfp = document.getElementById('valLpfp');
  const valLambda = document.getElementById('valLambda');

  const valMaf = document.getElementById('valMaf');
  const valIat = document.getElementById('valIat');
  const valTiming = document.getElementById('valTiming');
  const valLogSpeed = document.getElementById('valLogSpeed');

  const knockBars = [
    document.getElementById('knockBar1'),
    document.getElementById('knockBar2'),
    document.getElementById('knockBar3'),
    document.getElementById('knockBar4'),
    document.getElementById('knockBar5'),
    document.getElementById('knockBar6'),
  ];
  const valKnocks = [
    document.getElementById('valKnock1'),
    document.getElementById('valKnock2'),
    document.getElementById('valKnock3'),
    document.getElementById('valKnock4'),
    document.getElementById('valKnock5'),
    document.getElementById('valKnock6'),
  ];

  const valMisfires = [
    document.getElementById('valMisfire1'),
    document.getElementById('valMisfire2'),
    document.getElementById('valMisfire3'),
    document.getElementById('valMisfire4'),
    document.getElementById('valMisfire5'),
    document.getElementById('valMisfire6'),
  ];

  const valVanosInTgt = document.getElementById('valVanosInTgt');
  const valVanosInAct = document.getElementById('valVanosInAct');
  const valVanosExTgt = document.getElementById('valVanosExTgt');
  const valVanosExAct = document.getElementById('valVanosExAct');

  let datalogHistory = [];
  const MAX_DATALOG_POINTS = 120;
  let datalogPollInterval = null;
  let isRecordingDatalog = false;
  let lastExportedCsv = null;

  function drawDatalogChart() {
    if (!datalogCanvas) return;
    const ctx = datalogCanvas.getContext('2d');
    if (!ctx) return;

    const width = datalogCanvas.width;
    const height = datalogCanvas.height;

    // Clear background
    ctx.fillStyle = '#06080c';
    ctx.fillRect(0, 0, width, height);

    // Draw grid lines
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.06)';
    ctx.lineWidth = 1;

    // Horizontal grid (5 rows)
    for (let i = 1; i < 5; i++) {
      const y = (height / 5) * i;
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(width, y);
      ctx.stroke();
    }

    // Vertical grid (6 columns)
    for (let i = 1; i < 6; i++) {
      const x = (width / 6) * i;
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x, height);
      ctx.stroke();
    }

    if (datalogHistory.length < 2) return;

    function scaleBoost(psi) {
      const minPsi = -5;
      const maxPsi = 25;
      const norm = Math.max(0, Math.min(1, (psi - minPsi) / (maxPsi - minPsi)));
      return height - norm * (height - 20) - 10;
    }

    function scaleRpm(rpm) {
      const norm = Math.max(0, Math.min(1, rpm / 7000));
      return height - norm * (height - 20) - 10;
    }

    function scaleWgdc(wgdc) {
      const norm = Math.max(0, Math.min(1, wgdc / 100));
      return height - norm * (height - 20) - 10;
    }

    function scaleRail(bar) {
      const norm = Math.max(0, Math.min(1, bar / 220));
      return height - norm * (height - 20) - 10;
    }

    const n = datalogHistory.length;
    const stepX = width / (MAX_DATALOG_POINTS - 1);

    function drawTrace(getter, strokeStyle, lineWidth, isDashed = false) {
      ctx.strokeStyle = strokeStyle;
      ctx.lineWidth = lineWidth;
      if (isDashed) {
        ctx.setLineDash([4, 4]);
      } else {
        ctx.setLineDash([]);
      }
      ctx.beginPath();
      for (let i = 0; i < n; i++) {
        const x = i * stepX;
        const y = getter(datalogHistory[i]);
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      }
      ctx.stroke();
      ctx.setLineDash([]);
    }

    // 1. Engine RPM trace (Orange, faint background)
    drawTrace(d => scaleRpm(d.engine_rpm), 'rgba(255, 145, 0, 0.45)', 1.5);

    // 2. Rail Pressure trace (Purple)
    drawTrace(d => scaleRail(d.rail_pressure_actual_bar), '#e040fb', 1.8);

    // 3. WGDC % trace (Amber, dashed)
    drawTrace(d => scaleWgdc(d.wastegate_duty_percent), '#ffb300', 1.5, true);

    // 4. Boost Target trace (Cyan)
    drawTrace(d => scaleBoost(d.boost_target_psi), '#00b0ff', 2);

    // 5. Boost Actual trace (Bright Green)
    drawTrace(d => scaleBoost(d.boost_actual_psi), '#00e676', 2.5);
  }

  function renderDatalogFrame(frame) {
    if (!frame) return;

    // Push into ring buffer
    datalogHistory.push(frame);
    if (datalogHistory.length > MAX_DATALOG_POINTS) {
      datalogHistory.shift();
    }

    // Redraw graph
    drawDatalogChart();

    // Update legend
    if (legendTargetBoost) legendTargetBoost.innerText = `${frame.boost_target_psi.toFixed(1)} psi`;
    if (legendActualBoost) legendActualBoost.innerText = `${frame.boost_actual_psi.toFixed(1)} psi`;
    if (legendWgdc) legendWgdc.innerText = `${frame.wastegate_duty_percent.toFixed(0)}%`;
    if (legendRailPress) legendRailPress.innerText = `${frame.rail_pressure_actual_bar.toFixed(1)} bar`;
    if (legendRpm) legendRpm.innerText = `${frame.engine_rpm} RPM`;

    // Turbo tile
    if (valTargetBoost) valTargetBoost.innerText = `${frame.boost_target_psi.toFixed(1)} psi`;
    if (valTargetBoostHpa) valTargetBoostHpa.innerText = `${frame.boost_target_hpa.toFixed(0)} hPa`;
    if (valActualBoost) valActualBoost.innerText = `${frame.boost_actual_psi.toFixed(1)} psi`;
    if (valActualBoostHpa) valActualBoostHpa.innerText = `${frame.boost_actual_hpa.toFixed(0)} hPa`;

    const boostDelta = Math.abs(frame.boost_actual_psi - frame.boost_target_psi);
    if (boostDevBadge) {
      if (boostDelta < 1.0) {
        boostDevBadge.innerText = 'ON TARGET';
        boostDevBadge.style.backgroundColor = 'rgba(0, 230, 118, 0.12)';
        boostDevBadge.style.color = 'var(--accent-green)';
      } else if (boostDelta < 2.5) {
        boostDevBadge.innerText = `SPOOLING (Δ ${boostDelta.toFixed(1)} psi)`;
        boostDevBadge.style.backgroundColor = 'rgba(255, 179, 0, 0.12)';
        boostDevBadge.style.color = 'var(--accent-amber)';
      } else {
        boostDevBadge.innerText = `DEVIATION (Δ ${boostDelta.toFixed(1)} psi)`;
        boostDevBadge.style.backgroundColor = 'rgba(255, 23, 68, 0.12)';
        boostDevBadge.style.color = 'var(--accent-red)';
      }
    }

    if (valWgdcText) valWgdcText.innerText = `${frame.wastegate_duty_percent.toFixed(1)}%`;
    if (wgdcBarFill) wgdcBarFill.style.width = `${Math.min(Math.max(frame.wastegate_duty_percent, 0), 100)}%`;

    // Fuel tile
    if (valRailTarget) valRailTarget.innerText = `${frame.rail_pressure_target_bar.toFixed(1)} bar`;
    if (valRailTargetPsi) valRailTargetPsi.innerText = `(${Math.round(frame.rail_pressure_target_bar * 14.5038)} psi)`;
    if (valRailActual) valRailActual.innerText = `${frame.rail_pressure_actual_bar.toFixed(1)} bar`;
    if (valRailActualPsi) valRailActualPsi.innerText = `(${Math.round(frame.rail_pressure_actual_bar * 14.5038)} psi)`;

    const railDelta = frame.rail_pressure_target_bar - frame.rail_pressure_actual_bar;
    if (fuelHealthBadge) {
      if (railDelta > 20.0) {
        fuelHealthBadge.innerText = 'RAIL PRESSURE DIP';
        fuelHealthBadge.style.backgroundColor = 'rgba(255, 23, 68, 0.15)';
        fuelHealthBadge.style.color = 'var(--accent-red)';
      } else {
        fuelHealthBadge.innerText = 'HEALTHY';
        fuelHealthBadge.style.backgroundColor = 'rgba(0, 230, 118, 0.12)';
        fuelHealthBadge.style.color = 'var(--accent-green)';
      }
    }

    if (valLpfp) valLpfp.innerText = `${frame.low_pressure_fuel_bar.toFixed(1)} bar`;
    if (valLambda) valLambda.innerText = `${frame.lambda.toFixed(2)} λ (${(frame.lambda * 14.7).toFixed(1)} AFR)`;

    // Air & Ignition
    if (valMaf) valMaf.innerText = `${frame.mass_air_flow_gps.toFixed(1)} g/s`;
    if (valIat) valIat.innerText = `${frame.intake_air_temp_c.toFixed(1)} °C`;
    if (valTiming) valTiming.innerText = `${frame.ignition_timing_cyl1.toFixed(1)}° CRK`;
    if (valLogSpeed) valLogSpeed.innerText = `${frame.vehicle_speed_kmh} km/h`;

    // Knock Retard Cyl 1-6
    for (let c = 0; c < 6; c++) {
      const kr = frame.knock_retard[c] || 0.0;
      if (valKnocks[c]) {
        valKnocks[c].innerText = `${kr.toFixed(1)}°`;
        if (kr === 0.0) {
          valKnocks[c].style.color = 'var(--accent-green)';
        } else if (kr > -2.0) {
          valKnocks[c].style.color = 'var(--accent-amber)';
        } else {
          valKnocks[c].style.color = 'var(--accent-red)';
        }
      }
      if (knockBars[c]) {
        const pct = Math.min(Math.abs(kr) / 6.0, 1.0) * 100;
        knockBars[c].style.width = `${pct}%`;
        knockBars[c].style.backgroundColor = kr === 0 ? 'var(--accent-green)' : (kr > -2.0 ? 'var(--accent-amber)' : 'var(--accent-red)');
      }
    }

    // Misfires Cyl 1-6
    for (let m = 0; m < 6; m++) {
      const count = frame.misfire_counts[m] || 0;
      if (valMisfires[m]) {
        valMisfires[m].innerText = count;
        valMisfires[m].className = count === 0 ? 'misfire-val ok' : 'misfire-val fault';
      }
    }

    // VANOS
    if (valVanosInTgt) valVanosInTgt.innerText = `${frame.vanos_intake_target_deg.toFixed(1)}°`;
    if (valVanosInAct) valVanosInAct.innerText = `${frame.vanos_intake_actual_deg.toFixed(1)}°`;
    if (valVanosExTgt) valVanosExTgt.innerText = `${frame.vanos_exhaust_target_deg.toFixed(1)}°`;
    if (valVanosExAct) valVanosExAct.innerText = `${frame.vanos_exhaust_actual_deg.toFixed(1)}°`;
  }

  async function fetchDatalogFrame() {
    try {
      const frame = await callBackend('get_engine_datalog');
      if (datalogUnsupportedBanner) datalogUnsupportedBanner.style.display = 'none';
      renderDatalogFrame(frame);

      if (isRecordingDatalog) {
        const status = await callBackend('get_datalog_status');
        if (status && status.is_recording) {
          if (logRecordPill) {
            logRecordPill.innerText = `🔴 RECORDING (${status.elapsed_seconds.toFixed(1)}s, ${status.sample_count} samples)`;
            logRecordPill.className = 'datalog-record-pill recording';
          }
        }
      }
    } catch (err) {
      if (datalogUnsupportedBanner) datalogUnsupportedBanner.style.display = 'block';
    }
  }

  function startDatalogStream() {
    if (!datalogPollInterval) {
      datalogPollInterval = setInterval(fetchDatalogFrame, 80);
      fetchDatalogFrame();
    }
  }

  function stopDatalogStream() {
    if (datalogPollInterval && !isRecordingDatalog) {
      clearInterval(datalogPollInterval);
      datalogPollInterval = null;
    }
  }

  if (btnStartLog) {
    btnStartLog.addEventListener('click', async () => {
      try {
        await callBackend('start_datalog_recording');
        isRecordingDatalog = true;
        btnStartLog.style.display = 'none';
        btnStopLog.style.display = 'inline-block';
        if (btnDownloadCsv) btnDownloadCsv.style.display = 'none';
        if (logRecordPill) {
          logRecordPill.innerText = '🔴 RECORDING (0.0s, 0 samples)';
          logRecordPill.className = 'datalog-record-pill recording';
        }
        startDatalogStream();
      } catch (err) {
        alert('Failed to start datalog recording: ' + err);
      }
    });
  }

  if (btnStopLog) {
    btnStopLog.addEventListener('click', async () => {
      try {
        const summary = await callBackend('stop_datalog_recording');
        isRecordingDatalog = false;
        btnStartLog.style.display = 'inline-block';
        btnStopLog.style.display = 'none';
        if (btnDownloadCsv) btnDownloadCsv.style.display = 'inline-block';
        lastExportedCsv = summary.csv_content;
        if (logRecordPill) {
          const pathMsg = summary.saved_file_path ? ` - Saved to ${summary.saved_file_path}` : '';
          logRecordPill.innerText = `IDLE / EXPORTED (${summary.sample_count} samples in ${summary.duration_seconds.toFixed(1)}s${pathMsg})`;
          logRecordPill.className = 'datalog-record-pill';
        }
      } catch (err) {
        alert('Failed to stop recording: ' + err);
      }
    });
  }

  if (btnDownloadCsv) {
    btnDownloadCsv.addEventListener('click', () => {
      if (!lastExportedCsv) {
        alert('No exported CSV log available');
        return;
      }
      const blob = new Blob([lastExportedCsv], { type: 'text/csv;charset=utf-8;' });
      const url = URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.setAttribute('href', url);
      link.setAttribute('download', `bimmermon_n55_datalog_${Date.now()}.csv`);
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      URL.revokeObjectURL(url);
    });
  }

  // ====================================================
  // 6. VEHICLE CODING & CUSTOMIZATION CONTROLLER
  // ====================================================
  const codingVoltageBadge = document.getElementById('codingVoltageBadge');
  const btnRefreshCoding = document.getElementById('btnRefreshCoding');
  const btnOpenCodingBackups = document.getElementById('btnOpenCodingBackups');
  const codingBackupCount = document.getElementById('codingBackupCount');
  const codingCardsGrid = document.getElementById('codingCardsGrid');
  const countCodingAll = document.getElementById('countCodingAll');
  const codingFilterBtns = document.querySelectorAll('.coding-filter-btn');

  // Confirmation Modal Elements
  const codingConfirmModal = document.getElementById('codingConfirmModal');
  const btnCloseCodingConfirmX = document.getElementById('btnCloseCodingConfirmX');
  const btnCancelCodingConfirm = document.getElementById('btnCancelCodingConfirm');
  const btnExecuteCodingWrite = document.getElementById('btnExecuteCodingWrite');
  const confirmParamName = document.getElementById('confirmParamName');
  const confirmTargetEcu = document.getElementById('confirmTargetEcu');
  const confirmCurrentVal = document.getElementById('confirmCurrentVal');
  const confirmNewVal = document.getElementById('confirmNewVal');

  // Backups Modal Elements
  const codingBackupsModal = document.getElementById('codingBackupsModal');
  const btnCloseCodingBackupsX = document.getElementById('btnCloseCodingBackupsX');
  const btnCloseCodingBackups = document.getElementById('btnCloseCodingBackups');
  const codingBackupsList = document.getElementById('codingBackupsList');

  let codingParameters = [];
  let codingBackups = [];
  let activeCodingCategory = 'all';
  let pendingCodingWrite = null;

  function updateCodingVoltage(volts) {
    if (!codingVoltageBadge) return;
    const v = Number(volts) || 0;
    if (v >= 12.0) {
      codingVoltageBadge.className = 'coding-voltage-pill';
      codingVoltageBadge.innerText = `⚡ ${v.toFixed(1)}V Battery Safe`;
    } else {
      codingVoltageBadge.className = 'coding-voltage-pill warn';
      codingVoltageBadge.innerText = `⚠️ ${v.toFixed(1)}V Low Battery Voltage`;
    }
  }

  async function fetchCodingParameters() {
    if (!codingCardsGrid) return;
    try {
      // Query battery status in background to keep voltage pill accurate
      callBackend('get_battery_status').then(bat => {
        if (bat && bat.supported && typeof bat.voltage === 'number') {
          updateCodingVoltage(bat.voltage);
        }
      }).catch(() => {});

      // Refresh pre-coding backup count
      fetchCodingBackups(false);

      const params = await callBackend('get_coding_parameters');
      if (Array.isArray(params)) {
        codingParameters = params;
        renderCodingCards();
      }
    } catch (err) {
      console.error('Failed to load coding parameters:', err);
      if (codingCardsGrid) {
        codingCardsGrid.innerHTML = `
          <div class="cbs-card" style="grid-column: 1 / -1; text-align: center; color: var(--accent-red); padding: 30px;">
            ⚠️ Coding Unavailable: ${err}
          </div>
        `;
      }
    }
  }

  function renderCodingCards() {
    if (!codingCardsGrid) return;

    if (countCodingAll) {
      countCodingAll.innerText = codingParameters.length;
    }

    const filtered = activeCodingCategory === 'all'
      ? codingParameters
      : codingParameters.filter(p => p.category === activeCodingCategory);

    if (filtered.length === 0) {
      codingCardsGrid.innerHTML = `
        <div class="cbs-card" style="grid-column: 1 / -1; text-align: center; color: var(--text-dim); padding: 40px;">
          No coding parameters found for the selected category.
        </div>
      `;
      return;
    }

    codingCardsGrid.innerHTML = filtered.map(param => {
      const optionsHtml = param.options.map(opt => {
        const isSelected = opt.raw_value === param.active_raw_value;
        const oemTag = opt.is_factory_default ? ' [OEM]' : '';
        return `<option value="${opt.raw_value}" ${isSelected ? 'selected' : ''}>${opt.label}${oemTag}</option>`;
      }).join('');

      const hexAddr = '0x' + param.ecu_address.toString(16).toUpperCase().padStart(4, '0');

      return `
        <div class="coding-card" id="card-${param.id}">
          <div class="coding-card-header">
            <div class="coding-card-title-group">
              <span class="coding-cat-pill">${param.category}</span>
              <span class="coding-param-title">${param.name}</span>
            </div>
            <span class="coding-ecu-badge">${param.target_ecu} (${hexAddr})</span>
          </div>

          <div class="coding-desc">${param.description}</div>

          <div class="coding-active-row">
            <span>ACTIVE VALUE:</span>
            <strong class="coding-active-pill" id="active-val-${param.id}">${param.active_value}</strong>
          </div>

          <div class="coding-select-group">
            <label class="coding-select-label">TARGET CONFIGURATION</label>
            <select class="coding-select" id="select-${param.id}" data-param-id="${param.id}">
              ${optionsHtml}
            </select>
          </div>

          <div class="coding-card-footer">
            <span class="coding-card-status" id="status-${param.id}">Ready</span>
            <button class="btn btn-primary btn-sm btn-write-param" data-param-id="${param.id}">WRITE CODING</button>
          </div>
        </div>
      `;
    }).join('');

    // Attach click listeners to "WRITE CODING" buttons
    document.querySelectorAll('.btn-write-param').forEach(btn => {
      btn.addEventListener('click', () => {
        const paramId = btn.dataset.paramId;
        const param = codingParameters.find(p => p.id === paramId);
        if (!param) return;

        const selectElem = document.getElementById(`select-${param.id}`);
        if (!selectElem) return;

        const rawVal = parseInt(selectElem.value, 10);
        const selectedOpt = param.options.find(o => o.raw_value === rawVal);
        if (!selectedOpt) return;

        openCodingConfirmModal(param, selectedOpt);
      });
    });
  }

  function openCodingConfirmModal(param, selectedOpt) {
    pendingCodingWrite = {
      param,
      option: selectedOpt,
      cardId: `card-${param.id}`,
      statusId: `status-${param.id}`,
      activeValId: `active-val-${param.id}`,
    };

    const hexAddr = '0x' + param.ecu_address.toString(16).toUpperCase().padStart(4, '0');
    if (confirmParamName) confirmParamName.innerText = param.name;
    if (confirmTargetEcu) confirmTargetEcu.innerText = `${param.target_ecu} (${hexAddr})`;
    if (confirmCurrentVal) confirmCurrentVal.innerText = param.active_value;
    if (confirmNewVal) confirmNewVal.innerText = selectedOpt.label;

    if (codingConfirmModal) codingConfirmModal.style.display = 'flex';
  }

  async function executePendingCodingWrite() {
    if (!pendingCodingWrite) return;
    const { param, option, statusId, activeValId } = pendingCodingWrite;

    if (btnExecuteCodingWrite) {
      btnExecuteCodingWrite.disabled = true;
      btnExecuteCodingWrite.innerText = '⚡ WRITING & RESETTING ECU...';
    }

    try {
      const msg = await callBackend('write_coding_parameter', {
        paramId: param.id,
        rawValue: option.raw_value,
      });

      if (codingConfirmModal) codingConfirmModal.style.display = 'none';

      // Update card UI immediately
      const statusElem = document.getElementById(statusId);
      if (statusElem) {
        statusElem.innerText = '✓ Applied & Restarted';
        statusElem.className = 'coding-card-status success';
      }
      const activeElem = document.getElementById(activeValId);
      if (activeElem) {
        activeElem.innerText = option.label;
      }

      // Refresh parameter list and backups in background
      await fetchCodingParameters();
      await fetchCodingBackups(false);

      alert(msg || `Successfully coded ${param.name} to ${option.label}!`);
    } catch (err) {
      if (codingConfirmModal) codingConfirmModal.style.display = 'none';

      const statusElem = document.getElementById(statusId);
      if (statusElem) {
        statusElem.innerText = '✕ Write failed';
        statusElem.className = 'coding-card-status error';
      }

      alert(`Failed to write coding: ${err}`);
    } finally {
      if (btnExecuteCodingWrite) {
        btnExecuteCodingWrite.disabled = false;
        btnExecuteCodingWrite.innerText = '⚡ WRITE & APPLY CODING';
      }
      pendingCodingWrite = null;
    }
  }

  async function fetchCodingBackups(openModal = false) {
    try {
      const backups = await callBackend('get_coding_backups');
      if (Array.isArray(backups)) {
        codingBackups = backups;
        if (codingBackupCount) codingBackupCount.innerText = backups.length;
        renderCodingBackupsList();
      }
      if (openModal && codingBackupsModal) {
        codingBackupsModal.style.display = 'flex';
      }
    } catch (err) {
      console.error('Failed to load coding backups:', err);
    }
  }

  function renderCodingBackupsList() {
    if (!codingBackupsList) return;

    if (codingBackups.length === 0) {
      codingBackupsList.innerHTML = `
        <div class="cbs-card" style="text-align: center; color: var(--text-dim); padding: 30px;">
          No pre-coding backups found for the active vehicle profile.
        </div>
      `;
      return;
    }

    codingBackupsList.innerHTML = codingBackups.map(b => {
      const hexAddr = '0x' + b.ecu_address.toString(16).toUpperCase().padStart(4, '0');
      const timeStr = b.timestamp.replace('T', ' ').substring(0, 19);

      return `
        <div class="backup-item-card">
          <div class="backup-left">
            <div class="backup-header-line">
              <span class="backup-title">${b.param_name}</span>
              <span class="coding-ecu-badge">${b.ecu_name} (${hexAddr})</span>
            </div>
            <span class="backup-val-trans">
              Previous: <strong style="color: var(--accent-green);">${b.previous_value_label}</strong> → Written: <span style="color: var(--accent-cyan);">${b.new_value_label}</span>
            </span>
            <span class="backup-time">Saved: ${timeStr}</span>
          </div>
          <button class="btn btn-secondary btn-sm btn-restore-backup" data-backup-id="${b.id}">RESTORE</button>
        </div>
      `;
    }).join('');

    // Attach click listeners to restore buttons
    document.querySelectorAll('.btn-restore-backup').forEach(btn => {
      btn.addEventListener('click', async () => {
        const backupId = btn.dataset.backupId;
        const backup = codingBackups.find(b => b.id === backupId);
        if (!backup) return;

        if (!confirm(`Restore ${backup.param_name} back to "${backup.previous_value_label}" on ${backup.ecu_name}? This will perform an ECU soft reset.`)) {
          return;
        }

        btn.disabled = true;
        btn.innerText = 'Restoring...';

        try {
          const msg = await callBackend('restore_coding_backup', { backupId });
          alert(msg || 'Coding restored successfully!');
          await fetchCodingParameters();
          await fetchCodingBackups(false);
        } catch (err) {
          alert('Failed to restore coding: ' + err);
        } finally {
          btn.disabled = false;
          btn.innerText = 'RESTORE';
        }
      });
    });
  }

  // Filter Buttons
  codingFilterBtns.forEach(btn => {
    btn.addEventListener('click', () => {
      codingFilterBtns.forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      activeCodingCategory = btn.dataset.category || 'all';
      renderCodingCards();
    });
  });

  // Toolbar Buttons
  if (btnRefreshCoding) {
    btnRefreshCoding.addEventListener('click', () => {
      fetchCodingParameters();
    });
  }

  if (btnOpenCodingBackups) {
    btnOpenCodingBackups.addEventListener('click', () => {
      fetchCodingBackups(true);
    });
  }

  // Modal Closures
  if (btnCloseCodingConfirmX) {
    btnCloseCodingConfirmX.addEventListener('click', () => {
      if (codingConfirmModal) codingConfirmModal.style.display = 'none';
      pendingCodingWrite = null;
    });
  }

  if (btnCancelCodingConfirm) {
    btnCancelCodingConfirm.addEventListener('click', () => {
      if (codingConfirmModal) codingConfirmModal.style.display = 'none';
      pendingCodingWrite = null;
    });
  }

  if (btnExecuteCodingWrite) {
    btnExecuteCodingWrite.addEventListener('click', () => {
      executePendingCodingWrite();
    });
  }

  if (btnCloseCodingBackupsX) {
    btnCloseCodingBackupsX.addEventListener('click', () => {
      if (codingBackupsModal) codingBackupsModal.style.display = 'none';
    });
  }

  if (btnCloseCodingBackups) {
    btnCloseCodingBackups.addEventListener('click', () => {
      if (codingBackupsModal) codingBackupsModal.style.display = 'none';
    });
  }

  window.addEventListener('click', (e) => {
    if (e.target === codingConfirmModal) {
      codingConfirmModal.style.display = 'none';
      pendingCodingWrite = null;
    }
    if (e.target === codingBackupsModal) {
      codingBackupsModal.style.display = 'none';
    }
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

  // Initial immediate snapshot and garage queries
  callBackend('get_snapshot').then(snap => {
    if (snap) renderTelemetry(snap);
  }).catch(() => {});

  fetchGarage();
  fetchCodingBackups(false);
}

// Ensure execution when DOM is ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initApp);
} else {
  initApp();
}

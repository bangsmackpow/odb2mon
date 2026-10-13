use crate::error::ObdError;
use crate::profile::{
    BatteryStatus, CbsItem, CbsItemType, CbsReport, CbsStatus, DtcRecord, DtcStatus,
    EngineDatalogFrame, ParameterType, ParameterValue,
};

/// UDS Service 0x31 RoutineControl subfunctions
pub const ROUTINE_CONTROL_START: u8 = 0x01;
pub const ROUTINE_CONTROL_STOP: u8 = 0x02;
pub const ROUTINE_CONTROL_REQUEST_RESULTS: u8 = 0x03;

/// BMW Standard Routine Identifiers
pub const ROUTINE_BMW_BATTERY_REGISTRATION: u16 = 0x0219;
pub const ROUTINE_BMW_CBS_RESET: u16 = 0x0220;
pub const ROUTINE_BMW_WATER_PUMP_BLEED: u16 = 0x0211;
pub const ROUTINE_BMW_RADIATOR_FAN: u16 = 0x0215;
pub const ROUTINE_BMW_EXHAUST_FLAP: u16 = 0x0218;
pub const ROUTINE_BMW_EGS_OIL_LEVEL: u16 = 0x0225;

/// BMW Standard Diagnostic Identifiers (DIDs)
pub const DID_BMW_IBS_TELEMETRY: u16 = 0xD010;
pub const DID_BMW_BATTERY_HISTORY: u16 = 0x2036;
pub const DID_BMW_CBS_OVERVIEW: u16 = 0xD020;
pub const DID_BMW_WATER_PUMP_STATUS: u16 = 0x2042;
pub const DID_BMW_TRANSMISSION_SUMP_TEMP: u16 = 0x2045;
pub const DID_BMW_N55_DATALOG: u16 = 0xD100;

/// Builds UDS Service 0x22 ReadDataByIdentifier request frame.
pub fn build_read_did_request(did: u16) -> Vec<u8> {
    vec![0x22, (did >> 8) as u8, (did & 0xFF) as u8]
}

/// Builds UDS Service 0x10 DiagnosticSessionControl request frame.
/// 0x01: Default Session
/// 0x02: Programming Session
/// 0x03: Extended Diagnostic Session
pub fn build_session_control_request(session_type: u8) -> Vec<u8> {
    vec![0x10, session_type]
}

/// Builds UDS Service 0x04 ClearDiagnosticInformation request frame.
/// Group 0xFFFFFF clears all fault memories across all emissions and chassis domains.
pub fn build_clear_dtc_request() -> Vec<u8> {
    vec![0x04, 0xFF, 0xFF, 0xFF]
}

/// Builds UDS Service 0x31 RoutineControl request frame.
pub fn build_routine_control_request(
    subfunction: u8,
    routine_id: u16,
    option_record: &[u8],
) -> Vec<u8> {
    let mut req = Vec::with_capacity(4 + option_record.len());
    req.push(0x31);
    req.push(subfunction);
    req.push((routine_id >> 8) as u8);
    req.push((routine_id & 0xFF) as u8);
    req.extend_from_slice(option_record);
    req
}

/// Convenience helper to start a routine (0x31 0x01 <routine_id>).
pub fn build_start_routine(routine_id: u16, option_record: &[u8]) -> Vec<u8> {
    build_routine_control_request(ROUTINE_CONTROL_START, routine_id, option_record)
}

/// Convenience helper to stop a routine (0x31 0x02 <routine_id>).
pub fn build_stop_routine(routine_id: u16) -> Vec<u8> {
    build_routine_control_request(ROUTINE_CONTROL_STOP, routine_id, &[])
}

/// Builds Radiator Fan Actuator request (Routine 0x0215).
/// Duty percent: 0 (Off/Auto), 15 (Low), 50 (Med), 90 (High).
pub fn build_radiator_fan_request(duty: u8) -> Vec<u8> {
    build_start_routine(ROUTINE_BMW_RADIATOR_FAN, &[duty])
}

/// Builds Exhaust Flap Actuator request (Routine 0x0218).
/// Open: true (0x01), Closed: false (0x00).
pub fn build_exhaust_flap_request(open: bool) -> Vec<u8> {
    let state = if open { 0x01 } else { 0x00 };
    build_start_routine(ROUTINE_BMW_EXHAUST_FLAP, &[state])
}

/// Parses Transmission Sump Fluid Temperature from DID 0x2045 response payload.
pub fn parse_transmission_temp_response(payload: &[u8]) -> Result<f32, ObdError> {
    if payload.is_empty() {
        return Err(ObdError::ParseError(
            "Empty transmission temperature payload".into(),
        ));
    }
    if payload[0] == 0x7F {
        let nrc = payload.get(2).copied().unwrap_or(0x10);
        return Err(ObdError::from_nrc(
            payload.get(1).copied().unwrap_or(0x22),
            nrc,
        ));
    }
    if payload[0] != 0x62 {
        return Err(ObdError::Protocol(format!(
            "Unexpected response SID: {:#04X}, expected 0x62",
            payload[0]
        )));
    }
    if payload.len() < 4 {
        return Err(ObdError::ParseError(
            "Transmission temperature response truncated".into(),
        ));
    }
    let raw_temp = payload[3];
    let temp_c = (raw_temp as f32) - 40.0;
    Ok(temp_c)
}

/// Parses UDS Service 0x31 response frame.
/// Handles Positive Response (0x71) and Negative Response (0x7F).
pub fn parse_routine_response(routine_id: u16, payload: &[u8]) -> Result<Vec<u8>, ObdError> {
    if payload.is_empty() {
        return Err(ObdError::ParseError(
            "Empty UDS RoutineControl payload".into(),
        ));
    }

    if payload[0] == 0x7F {
        if payload.len() < 3 {
            return Err(ObdError::ParseError(
                "Malformed UDS negative response".into(),
            ));
        }
        let service_id = payload[1];
        let nrc = payload[2];
        return Err(ObdError::from_nrc(service_id, nrc));
    }

    if payload[0] != 0x71 {
        return Err(ObdError::Protocol(format!(
            "Unexpected routine response SID: {:#04X}, expected 0x71",
            payload[0]
        )));
    }

    if payload.len() < 4 {
        return Err(ObdError::ParseError(
            "Routine response frame truncated".into(),
        ));
    }

    let returned_id = u16::from_be_bytes([payload[2], payload[3]]);
    if returned_id != routine_id {
        return Err(ObdError::Protocol(format!(
            "Routine ID mismatch: expected {routine_id:#04X}, received {returned_id:#04X}"
        )));
    }

    Ok(payload[4..].to_vec())
}

/// Parses BMW Intelligent Battery Sensor (IBS) telemetry from DID 0xD010 / 0x2035 payload.
/// Format:
/// - Byte 0: State of Charge (SoC) % (0 - 100)
/// - Bytes 1-2: Battery Terminal Voltage in mV (e.g. 12600 -> 12.60 V)
/// - Bytes 3-4: Battery Net Current in signed 0.1 A (e.g. -142 -> -14.2 A)
/// - Byte 5: Temperature in °C (offset by -40°C, so raw - 40)
/// - Byte 6: Capacity in Ah (e.g. 90)
/// - Byte 7: Battery Chemistry (0x01 = "AGM", 0x02 = "Lead-Acid / Wet", 0x03 = "Lithium")
pub fn parse_battery_telemetry_response(
    expected_did: u16,
    payload: &[u8],
) -> Result<BatteryStatus, ObdError> {
    if payload.is_empty() {
        return Err(ObdError::ParseError(
            "Empty battery telemetry payload".into(),
        ));
    }

    if payload[0] == 0x7F {
        let nrc = payload.get(2).copied().unwrap_or(0x10);
        return Err(ObdError::from_nrc(
            payload.get(1).copied().unwrap_or(0x22),
            nrc,
        ));
    }

    if payload[0] != 0x62 {
        return Err(ObdError::Protocol(format!(
            "Unexpected response SID: {:#04X}, expected 0x62",
            payload[0]
        )));
    }

    if payload.len() < 3 {
        return Err(ObdError::ParseError(
            "Battery telemetry response truncated".into(),
        ));
    }

    let returned_did = u16::from_be_bytes([payload[1], payload[2]]);
    if returned_did != expected_did {
        return Err(ObdError::Protocol(format!(
            "DID mismatch: expected {expected_did:#04X}, received {returned_did:#04X}"
        )));
    }

    let data = &payload[3..];
    if data.len() < 6 {
        return Err(ObdError::ParseError(
            "IBS telemetry requires at least 6 bytes".into(),
        ));
    }

    let soc = (data[0] as f32).clamp(0.0, 100.0);
    let voltage = (u16::from_be_bytes([data[1], data[2]]) as f32) / 1000.0;
    let current_raw = i16::from_be_bytes([data[3], data[4]]);
    let current = (current_raw as f32) / 10.0;
    let temp = (data[5] as f32) - 40.0;

    let capacity = if data.len() > 6 && data[6] > 0 {
        Some(data[6] as u32)
    } else {
        Some(90)
    };

    let battery_type = if data.len() > 7 {
        match data[7] {
            0x01 => Some("AGM".into()),
            0x02 => Some("Lead-Acid / Wet".into()),
            0x03 => Some("Lithium / LiFePO4".into()),
            _ => Some("AGM (Standard)".into()),
        }
    } else {
        Some("AGM".into())
    };

    Ok(BatteryStatus {
        supported: true,
        state_of_charge_percent: soc,
        voltage,
        current_amps: current,
        temperature_celsius: temp,
        capacity_ah: capacity,
        battery_type,
        last_replacement_km: Vec::new(),
    })
}

/// Parses battery replacement history from DID 0x2036.
/// Contains up to 3 odometer readings in kilometers (4 bytes big-endian each).
pub fn parse_battery_history_response(payload: &[u8]) -> Result<Vec<u32>, ObdError> {
    if payload.is_empty() {
        return Ok(Vec::new());
    }

    if payload[0] == 0x7F {
        let nrc = payload.get(2).copied().unwrap_or(0x10);
        return Err(ObdError::from_nrc(
            payload.get(1).copied().unwrap_or(0x22),
            nrc,
        ));
    }

    if payload[0] != 0x62 || payload.len() < 3 {
        return Ok(Vec::new());
    }

    let mut history = Vec::new();
    let data = &payload[3..];

    for chunk in data.as_chunks::<4>().0 {
        let km = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        if km > 0 {
            history.push(km);
        }
    }

    Ok(history)
}

/// Builds UDS Service 0x31 request to reset a BMW Condition Based Service (CBS) item.
pub fn build_cbs_reset_routine(item_type: CbsItemType) -> Vec<u8> {
    build_start_routine(ROUTINE_BMW_CBS_RESET, &[item_type.id_byte()])
}

/// Parses UDS Service 0x22 response payload for BMW Condition Based Service (CBS) overview (DID 0xD020).
pub fn parse_cbs_report_response(expected_did: u16, payload: &[u8]) -> Result<CbsReport, ObdError> {
    if payload.is_empty() {
        return Err(ObdError::ParseError("Empty CBS payload".into()));
    }

    if payload[0] == 0x7F {
        let nrc = payload.get(2).copied().unwrap_or(0x10);
        return Err(ObdError::from_nrc(
            payload.get(1).copied().unwrap_or(0x22),
            nrc,
        ));
    }

    if payload[0] != 0x62 {
        return Err(ObdError::Protocol(format!(
            "Unexpected response SID: {:#04X}, expected 0x62",
            payload[0]
        )));
    }

    if payload.len() < 3 {
        return Err(ObdError::ParseError("CBS response truncated".into()));
    }

    let returned_did = u16::from_be_bytes([payload[1], payload[2]]);
    if returned_did != expected_did {
        return Err(ObdError::Protocol(format!(
            "DID mismatch: expected {expected_did:#04X}, received {returned_did:#04X}"
        )));
    }

    let data = &payload[3..];
    let mut items = Vec::new();

    // Each CBS record is 8 bytes
    for chunk in data.as_chunks::<8>().0 {
        if let Some(item_type) = CbsItemType::from_id_byte(chunk[0]) {
            let status = match chunk[1] {
                0x00 => CbsStatus::Ok,
                0x01 => CbsStatus::Warning,
                _ => CbsStatus::Overdue,
            };

            let remaining_percent = (chunk[2] as f32).clamp(0.0, 100.0);

            let km_raw = u16::from_be_bytes([chunk[3], chunk[4]]);
            let remaining_km = if km_raw == 0xFFFF {
                None
            } else {
                Some((km_raw as i32) * 10)
            };

            let due_month = chunk[5];
            let due_year_offset = chunk[6];
            let due_date = if due_month > 0 && due_year_offset > 0 {
                Some(format!(
                    "{:02}/{:04}",
                    due_month,
                    2000 + due_year_offset as u32
                ))
            } else {
                None
            };

            let reset_count = chunk[7] as u32;

            items.push(CbsItem {
                name: item_type.display_name().to_string(),
                item_type,
                status,
                remaining_percent,
                remaining_km,
                due_date,
                reset_count,
            });
        }
    }

    Ok(CbsReport {
        supported: true,
        items,
    })
}

/// Parses UDS Service 0x22 response frame.
/// Handles Positive Response (0x62), Negative Response (0x7F), and NRC parsing.
pub fn parse_did_response(
    expected_did: u16,
    param: ParameterType,
    payload: &[u8],
) -> Result<ParameterValue, ObdError> {
    if payload.is_empty() {
        return Err(ObdError::ParseError("Empty UDS payload".into()));
    }

    // Check for Negative Response Code (NRC)
    if payload[0] == 0x7F {
        if payload.len() < 3 {
            return Err(ObdError::ParseError(
                "Malformed UDS negative response".into(),
            ));
        }
        let service_id = payload[1];
        let nrc = payload[2];
        return Err(ObdError::from_nrc(service_id, nrc));
    }

    // Check for Positive Response (0x22 + 0x40 = 0x62)
    if payload[0] != 0x62 {
        return Err(ObdError::Protocol(format!(
            "Unexpected response service ID: {:#04X}, expected 0x62",
            payload[0]
        )));
    }

    if payload.len() < 3 {
        return Err(ObdError::ParseError("Response frame truncated".into()));
    }

    let returned_did = u16::from_be_bytes([payload[1], payload[2]]);
    if returned_did != expected_did {
        return Err(ObdError::Protocol(format!(
            "DID mismatch: expected {expected_did:#04X}, received {returned_did:#04X}"
        )));
    }

    let data = &payload[3..];
    match param {
        ParameterType::EngineRpm => {
            // BMW N55 2-byte RPM DID: (A * 256 + B) / 4
            if data.len() < 2 {
                return Err(ObdError::ParseError(
                    "RPM data requires at least 2 bytes".into(),
                ));
            }
            let raw = u16::from_be_bytes([data[0], data[1]]) as u32;
            Ok(ParameterValue::Rpm(raw / 4))
        }
        ParameterType::VehicleSpeed => {
            if data.is_empty() {
                return Err(ObdError::ParseError(
                    "Speed data requires at least 1 byte".into(),
                ));
            }
            Ok(ParameterValue::SpeedKmh(data[0] as u32))
        }
        ParameterType::CoolantTemperature => {
            if data.is_empty() {
                return Err(ObdError::ParseError(
                    "Coolant temp requires at least 1 byte".into(),
                ));
            }
            // BMW standard formula: raw - 48°C
            let temp = (data[0] as f32) - 48.0;
            Ok(ParameterValue::TemperatureCelsius(temp))
        }
        ParameterType::OilTemperature => {
            if data.is_empty() {
                return Err(ObdError::ParseError(
                    "Oil temp requires at least 1 byte".into(),
                ));
            }
            let temp = (data[0] as f32) - 48.0;
            Ok(ParameterValue::TemperatureCelsius(temp))
        }
    }
}

/// Parses Service 0x19 0x02 DTC response payload into structured DtcRecords.
pub fn parse_dtc_response(payload: &[u8]) -> Result<Vec<DtcRecord>, ObdError> {
    if payload.is_empty() {
        return Ok(vec![]);
    }

    if payload[0] == 0x7F {
        let nrc = payload.get(2).copied().unwrap_or(0x10);
        return Err(ObdError::from_nrc(
            payload.get(1).copied().unwrap_or(0x19),
            nrc,
        ));
    }

    // Positive response to 0x19 is 0x59
    if payload[0] != 0x59 {
        return Ok(vec![]);
    }

    if payload.len() < 3 {
        return Ok(vec![]);
    }

    let mut dtcs = Vec::new();
    let dtc_data = &payload[3..]; // Skip SID, Subfunction, and StatusAvailabilityMask

    // Each DTC record in ISO 14229 is 4 bytes: 3 bytes DTC Code + 1 byte Status
    for chunk in dtc_data.as_chunks::<4>().0 {
        let code_hex = format!("{:02X}{:02X}{:02X}", chunk[0], chunk[1], chunk[2]);
        let description = crate::dtc_db::lookup_description(&code_hex).to_string();
        let status_byte = chunk[3];
        let status = if (status_byte & 0x08) != 0 {
            DtcStatus::Confirmed
        } else if (status_byte & 0x04) != 0 {
            DtcStatus::Pending
        } else {
            DtcStatus::Permanent
        };

        dtcs.push(DtcRecord {
            code: code_hex,
            description,
            status,
            ecu_source: Some("BMW DME".into()),
        });
    }

    Ok(dtcs)
}

/// Parses BMW N55 high-speed engine datalog frame from DID 0xD100 payload.
pub fn parse_n55_datalog_response(payload: &[u8]) -> Result<EngineDatalogFrame, ObdError> {
    if payload.is_empty() {
        return Err(ObdError::ParseError("Empty datalog payload".into()));
    }
    if payload[0] == 0x7F {
        let nrc = payload.get(2).copied().unwrap_or(0x10);
        return Err(ObdError::from_nrc(
            payload.get(1).copied().unwrap_or(0x22),
            nrc,
        ));
    }
    if payload[0] != 0x62 {
        return Err(ObdError::Protocol(format!(
            "Unexpected response SID: {:#04X}, expected 0x62",
            payload[0]
        )));
    }
    if payload.len() < 3 + 38 {
        return Err(ObdError::ParseError(format!(
            "Datalog payload truncated: expected at least 41 bytes, got {}",
            payload.len()
        )));
    }

    let returned_did = u16::from_be_bytes([payload[1], payload[2]]);
    if returned_did != DID_BMW_N55_DATALOG {
        return Err(ObdError::Protocol(format!(
            "DID mismatch: expected {:#04X}, received {:#04X}",
            DID_BMW_N55_DATALOG, returned_did
        )));
    }

    let d = &payload[3..];
    let rpm = u16::from_be_bytes([d[0], d[1]]);
    let speed = d[2];
    let boost_tgt_hpa = u16::from_be_bytes([d[3], d[4]]) as f32;
    let boost_act_hpa = u16::from_be_bytes([d[5], d[6]]) as f32;
    let boost_tgt_psi = (boost_tgt_hpa - 1013.25) * 0.0145038;
    let boost_act_psi = (boost_act_hpa - 1013.25) * 0.0145038;
    let wgdc = d[7] as f32;
    let rail_tgt = (u16::from_be_bytes([d[8], d[9]]) as f32) / 10.0;
    let rail_act = (u16::from_be_bytes([d[10], d[11]]) as f32) / 10.0;
    let lpfp = (d[12] as f32) / 10.0;
    let lambda = (d[13] as f32) / 100.0;
    let iat = (d[14] as f32) - 40.0;
    let maf = (u16::from_be_bytes([d[15], d[16]]) as f32) / 10.0;
    let timing = (d[17] as f32 - 80.0) * 0.5;

    let knock_retard = [
        (d[18] as f32) * -0.1,
        (d[19] as f32) * -0.1,
        (d[20] as f32) * -0.1,
        (d[21] as f32) * -0.1,
        (d[22] as f32) * -0.1,
        (d[23] as f32) * -0.1,
    ];

    let misfire_counts = [
        d[24] as u16,
        d[25] as u16,
        d[26] as u16,
        d[27] as u16,
        d[28] as u16,
        d[29] as u16,
    ];

    let vanos_in_tgt = (u16::from_be_bytes([d[30], d[31]]) as f32) / 10.0;
    let vanos_in_act = (u16::from_be_bytes([d[32], d[33]]) as f32) / 10.0;
    let vanos_ex_tgt = (u16::from_be_bytes([d[34], d[35]]) as f32) / 10.0;
    let vanos_ex_act = (u16::from_be_bytes([d[36], d[37]]) as f32) / 10.0;

    Ok(EngineDatalogFrame {
        timestamp_ms: 0,
        engine_rpm: rpm,
        vehicle_speed_kmh: speed,
        boost_target_hpa: boost_tgt_hpa,
        boost_actual_hpa: boost_act_hpa,
        boost_target_psi: boost_tgt_psi,
        boost_actual_psi: boost_act_psi,
        wastegate_duty_percent: wgdc,
        rail_pressure_target_bar: rail_tgt,
        rail_pressure_actual_bar: rail_act,
        low_pressure_fuel_bar: lpfp,
        lambda,
        intake_air_temp_c: iat,
        mass_air_flow_gps: maf,
        ignition_timing_cyl1: timing,
        knock_retard,
        misfire_counts,
        vanos_intake_target_deg: vanos_in_tgt,
        vanos_intake_actual_deg: vanos_in_act,
        vanos_exhaust_target_deg: vanos_ex_tgt,
        vanos_exhaust_actual_deg: vanos_ex_act,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rpm_positive() {
        // Positive response 0x62, DID 0xD001, RPM data = 0x2710 (10000 / 4 = 2500 RPM)
        let payload = [0x62, 0xD0, 0x01, 0x27, 0x10];
        let res = parse_did_response(0xD001, ParameterType::EngineRpm, &payload).unwrap();
        assert_eq!(res, ParameterValue::Rpm(2500));
    }

    #[test]
    fn test_parse_coolant_temp() {
        // Positive response 0x62, DID 0xD003, Coolant = 140 (140 - 48 = 92°C)
        let payload = [0x62, 0xD0, 0x03, 140];
        let res = parse_did_response(0xD003, ParameterType::CoolantTemperature, &payload).unwrap();
        assert_eq!(res, ParameterValue::TemperatureCelsius(92.0));
    }

    #[test]
    fn test_parse_nrc_conditions_not_correct() {
        // NRC 0x22 (ConditionsNotCorrect)
        let payload = [0x7F, 0x22, 0x22];
        let err = parse_did_response(0xD001, ParameterType::EngineRpm, &payload).unwrap_err();
        match err {
            ObdError::NegativeResponse {
                service_id, nrc, ..
            } => {
                assert_eq!(service_id, 0x22);
                assert_eq!(nrc, 0x22);
            }
            _ => panic!("Expected NegativeResponse"),
        }
    }

    #[test]
    fn test_parse_dtc_response() {
        // Positive response 0x59, Subfunction 0x02, AvailabilityMask 0xFF
        // DTC 1: 0x12, 0x04, 0x08, Status 0x2F (bit 3 set -> Confirmed) -> 120408
        // DTC 2: 0x13, 0x01, 0x04, Status 0x24 (bit 2 set -> Pending)   -> 130104
        let payload = [
            0x59, 0x02, 0xFF, 0x12, 0x04, 0x08, 0x2F, 0x13, 0x01, 0x04, 0x24,
        ];
        let dtcs = parse_dtc_response(&payload).unwrap();
        assert_eq!(dtcs.len(), 2);
        assert_eq!(dtcs[0].code, "120408");
        assert_eq!(
            dtcs[0].description,
            "Charging pressure control: Switch-off, pressure build-up disabled"
        );
        assert_eq!(dtcs[0].status, DtcStatus::Confirmed);

        assert_eq!(dtcs[1].code, "130104");
        assert_eq!(
            dtcs[1].description,
            "VANOS, intake: Camshaft angle not reached"
        );
        assert_eq!(dtcs[1].status, DtcStatus::Pending);
    }

    #[test]
    fn test_routine_control_build_and_parse() {
        let req = build_start_routine(ROUTINE_BMW_BATTERY_REGISTRATION, &[0x01, 0x02]);
        assert_eq!(req, vec![0x31, 0x01, 0x02, 0x19, 0x01, 0x02]);

        // Positive response 0x71 0x01 0x02 0x19 with status byte 0x00
        let resp = [0x71, 0x01, 0x02, 0x19, 0x00];
        let res = parse_routine_response(ROUTINE_BMW_BATTERY_REGISTRATION, &resp).unwrap();
        assert_eq!(res, vec![0x00]);
    }

    #[test]
    fn test_routine_control_negative_response() {
        // NRC 0x22 ConditionsNotCorrect for Service 0x31
        let resp = [0x7F, 0x31, 0x22];
        let err = parse_routine_response(ROUTINE_BMW_BATTERY_REGISTRATION, &resp).unwrap_err();
        match err {
            ObdError::NegativeResponse {
                service_id, nrc, ..
            } => {
                assert_eq!(service_id, 0x31);
                assert_eq!(nrc, 0x22);
            }
            _ => panic!("Expected NegativeResponse"),
        }
    }

    #[test]
    fn test_parse_battery_telemetry() {
        // Positive response 0x62, DID 0xD010:
        // SoC = 88% (0x58), Voltage = 12650 mV (0x31, 0x6A), Current = -142 (-14.2A, 0xFF, 0x72)
        // Temp = 68 (68 - 40 = 28°C), Capacity = 90 Ah, Type = 0x01 (AGM)
        let payload = [0x62, 0xD0, 0x10, 0x58, 0x31, 0x6A, 0xFF, 0x72, 68, 90, 0x01];
        let status = parse_battery_telemetry_response(0xD010, &payload).unwrap();
        assert!(status.supported);
        assert_eq!(status.state_of_charge_percent, 88.0);
        assert!((status.voltage - 12.65).abs() < 0.01);
        assert!((status.current_amps - (-14.2)).abs() < 0.01);
        assert_eq!(status.temperature_celsius, 28.0);
        assert_eq!(status.capacity_ah, Some(90));
        assert_eq!(status.battery_type, Some("AGM".into()));
    }

    #[test]
    fn test_parse_battery_history() {
        // Positive response 0x62, DID 0x2036:
        // Entry 1: 142300 km (0x00, 0x02, 0x2B, 0xDC)
        // Entry 2: 78100 km (0x00, 0x01, 0x31, 0x14)
        let payload = [
            0x62, 0x20, 0x36, 0x00, 0x02, 0x2B, 0xDC, 0x00, 0x01, 0x31, 0x14,
        ];
        let history = parse_battery_history_response(&payload).unwrap();
        assert_eq!(history, vec![142300, 78100]);
    }

    #[test]
    fn test_cbs_reset_routine_build() {
        let req = build_cbs_reset_routine(CbsItemType::EngineOil);
        assert_eq!(req, vec![0x31, 0x01, 0x02, 0x20, 0x01]);

        let req_rear_brakes = build_cbs_reset_routine(CbsItemType::RearBrakes);
        assert_eq!(req_rear_brakes, vec![0x31, 0x01, 0x02, 0x20, 0x03]);
    }

    #[test]
    fn test_parse_cbs_report() {
        // Positive response 0x62, DID 0xD020:
        // Record 1 (Engine Oil): ID 0x01, Status 0x00 (Ok), 45% (0x2D), 620 (*10 = 6,200 km: 0x02, 0x6C), Month 11, Year 26 (2026), Counter 3
        // Record 2 (Rear Brakes): ID 0x03, Status 0x01 (Warning), 15% (0x0F), 310 (*10 = 3,100 km: 0x01, 0x36), Month 0, Year 0, Counter 1
        let payload = [
            0x62, 0xD0, 0x20, 0x01, 0x00, 0x2D, 0x02, 0x6C, 11, 26, 3, 0x03, 0x01, 0x0F, 0x01,
            0x36, 0, 0, 1,
        ];
        let report = parse_cbs_report_response(0xD020, &payload).unwrap();
        assert!(report.supported);
        assert_eq!(report.items.len(), 2);

        let oil = &report.items[0];
        assert_eq!(oil.item_type, CbsItemType::EngineOil);
        assert_eq!(oil.status, CbsStatus::Ok);
        assert_eq!(oil.remaining_percent, 45.0);
        assert_eq!(oil.remaining_km, Some(6200));
        assert_eq!(oil.due_date, Some("11/2026".into()));
        assert_eq!(oil.reset_count, 3);

        let rear_brakes = &report.items[1];
        assert_eq!(rear_brakes.item_type, CbsItemType::RearBrakes);
        assert_eq!(rear_brakes.status, CbsStatus::Warning);
        assert_eq!(rear_brakes.remaining_percent, 15.0);
        assert_eq!(rear_brakes.remaining_km, Some(3100));
        assert_eq!(rear_brakes.due_date, None);
        assert_eq!(rear_brakes.reset_count, 1);
    }

    #[test]
    fn test_parse_n55_datalog_frame() {
        // Construct 41-byte response:
        // 0x62, 0xD1, 0x00
        // RPM: 3500 (0x0D, 0xAC)
        // Speed: 85 km/h (0x55)
        // Boost target: 1850 hPa (0x07, 0x3A)
        // Boost actual: 1820 hPa (0x07, 0x1C)
        // WGDC: 68% (0x44)
        // Rail tgt: 180.0 bar -> 1800 (0x07, 0x08)
        // Rail act: 178.5 bar -> 1785 (0x06, 0xF9)
        // LPFP: 5.2 bar -> 52 (0x34)
        // Lambda: 0.88 -> 88 (0x58)
        // IAT: 35°C -> 75 (0x4B)
        // MAF: 165.0 g/s -> 1650 (0x06, 0x72)
        // Timing: 10.0 deg -> raw = 10.0/0.5 + 80 = 100 (0x64)
        // Knock retard: [0, -1.5, 0, 0, -0.8, 0] -> [0, 15, 0, 0, 8, 0]
        // Misfires: [0, 0, 0, 0, 0, 0]
        // VANOS IN tgt: 110.0 deg -> 1100 (0x04, 0x4C)
        // VANOS IN act: 109.5 deg -> 1095 (0x04, 0x47)
        // VANOS EX tgt: 105.0 deg -> 1050 (0x04, 0x1A)
        // VANOS EX act: 104.8 deg -> 1048 (0x04, 0x18)
        let mut payload = vec![0x62, 0xD1, 0x00];
        payload.extend_from_slice(&3500u16.to_be_bytes()); // RPM
        payload.push(85); // Speed
        payload.extend_from_slice(&1850u16.to_be_bytes()); // Boost tgt
        payload.extend_from_slice(&1820u16.to_be_bytes()); // Boost act
        payload.push(68); // WGDC
        payload.extend_from_slice(&1800u16.to_be_bytes()); // Rail tgt
        payload.extend_from_slice(&1785u16.to_be_bytes()); // Rail act
        payload.push(52); // LPFP
        payload.push(88); // Lambda
        payload.push(75); // IAT (75 - 40 = 35)
        payload.extend_from_slice(&1650u16.to_be_bytes()); // MAF
        payload.push(100); // Timing
        payload.extend_from_slice(&[0, 15, 0, 0, 8, 0]); // Knock retard
        payload.extend_from_slice(&[0, 0, 0, 0, 0, 0]); // Misfires
        payload.extend_from_slice(&1100u16.to_be_bytes()); // VANOS in tgt
        payload.extend_from_slice(&1095u16.to_be_bytes()); // VANOS in act
        payload.extend_from_slice(&1050u16.to_be_bytes()); // VANOS ex tgt
        payload.extend_from_slice(&1048u16.to_be_bytes()); // VANOS ex act

        assert_eq!(payload.len(), 41);
        let frame = parse_n55_datalog_response(&payload).unwrap();

        assert_eq!(frame.engine_rpm, 3500);
        assert_eq!(frame.vehicle_speed_kmh, 85);
        assert_eq!(frame.boost_target_hpa, 1850.0);
        assert_eq!(frame.boost_actual_hpa, 1820.0);
        assert!((frame.boost_target_psi - 12.13).abs() < 0.1);
        assert_eq!(frame.wastegate_duty_percent, 68.0);
        assert_eq!(frame.rail_pressure_target_bar, 180.0);
        assert_eq!(frame.rail_pressure_actual_bar, 178.5);
        assert_eq!(frame.low_pressure_fuel_bar, 5.2);
        assert!((frame.lambda - 0.88).abs() < 0.01);
        assert_eq!(frame.intake_air_temp_c, 35.0);
        assert_eq!(frame.mass_air_flow_gps, 165.0);
        assert_eq!(frame.ignition_timing_cyl1, 10.0);
        assert_eq!(frame.knock_retard[1], -1.5);
        assert_eq!(frame.knock_retard[4], -0.8);
        assert_eq!(frame.vanos_intake_target_deg, 110.0);
        assert_eq!(frame.vanos_intake_actual_deg, 109.5);
    }
}

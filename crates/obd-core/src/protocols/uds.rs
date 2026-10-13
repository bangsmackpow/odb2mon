use crate::error::ObdError;
use crate::profile::{BatteryStatus, DtcRecord, DtcStatus, ParameterType, ParameterValue};

/// UDS Service 0x31 RoutineControl subfunctions
pub const ROUTINE_CONTROL_START: u8 = 0x01;
pub const ROUTINE_CONTROL_STOP: u8 = 0x02;
pub const ROUTINE_CONTROL_REQUEST_RESULTS: u8 = 0x03;

/// BMW Standard Routine Identifiers
pub const ROUTINE_BMW_BATTERY_REGISTRATION: u16 = 0x0219;

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
}

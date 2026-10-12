use crate::error::ObdError;
use crate::profile::{DtcRecord, DtcStatus, ParameterType, ParameterValue};

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
            description: "BMW DME Hex Diagnostic Trouble Code".into(),
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
}

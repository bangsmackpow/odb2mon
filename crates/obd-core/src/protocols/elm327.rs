use crate::error::ObdError;
use crate::profile::{DtcRecord, DtcStatus, ParameterValue};

/// Sanitizes ELM327 ASCII responses by stripping spaces, prompt symbols, and linefeeds.
pub fn sanitize_response(raw: &str) -> String {
    raw.chars()
        .filter(|c| !c.is_whitespace() && *c != '>')
        .collect::<String>()
        .to_uppercase()
}

/// Parses SAE J1979 Mode 01 PID 0C (Engine RPM).
/// Formula: ((A * 256) + B) / 4
pub fn parse_mode01_rpm(raw_response: &str) -> Result<ParameterValue, ObdError> {
    let clean = sanitize_response(raw_response);
    if clean.contains("NODATA") || clean.contains("?") {
        return Ok(ParameterValue::NotAvailable);
    }

    let pos = clean.find("410C").ok_or_else(|| {
        ObdError::ParseError(format!("Invalid Mode 01 PID 0C response: {raw_response}"))
    })?;

    let bytes_hex = &clean[pos + 4..];
    if bytes_hex.len() < 4 {
        return Err(ObdError::ParseError("Truncated RPM hex payload".into()));
    }

    let a = u32::from_str_radix(&bytes_hex[0..2], 16)
        .map_err(|e| ObdError::ParseError(e.to_string()))?;
    let b = u32::from_str_radix(&bytes_hex[2..4], 16)
        .map_err(|e| ObdError::ParseError(e.to_string()))?;
    let rpm = ((a * 256) + b) / 4;

    Ok(ParameterValue::Rpm(rpm))
}

/// Parses SAE J1979 Mode 01 PID 0D (Vehicle Speed).
/// Formula: A (km/h)
pub fn parse_mode01_speed(raw_response: &str) -> Result<ParameterValue, ObdError> {
    let clean = sanitize_response(raw_response);
    let pos = clean.find("410D").ok_or_else(|| {
        ObdError::ParseError(format!("Invalid Mode 01 PID 0D response: {raw_response}"))
    })?;

    let bytes_hex = &clean[pos + 4..];
    if bytes_hex.len() < 2 {
        return Err(ObdError::ParseError("Truncated Speed hex payload".into()));
    }

    let speed = u32::from_str_radix(&bytes_hex[0..2], 16)
        .map_err(|e| ObdError::ParseError(e.to_string()))?;
    Ok(ParameterValue::SpeedKmh(speed))
}

/// Parses SAE J1979 Mode 01 PID 05 (Engine Coolant Temperature).
/// Formula: A - 40 (°C)
pub fn parse_mode01_coolant_temp(raw_response: &str) -> Result<ParameterValue, ObdError> {
    let clean = sanitize_response(raw_response);
    let pos = clean.find("4105").ok_or_else(|| {
        ObdError::ParseError(format!("Invalid Mode 01 PID 05 response: {raw_response}"))
    })?;

    let bytes_hex = &clean[pos + 4..];
    if bytes_hex.len() < 2 {
        return Err(ObdError::ParseError("Truncated Coolant hex payload".into()));
    }

    let raw = u32::from_str_radix(&bytes_hex[0..2], 16)
        .map_err(|e| ObdError::ParseError(e.to_string()))?;
    let temp = (raw as f32) - 40.0;
    Ok(ParameterValue::TemperatureCelsius(temp))
}

/// Parses GM Manufacturer Specific Mode 22 PID 1154 (Engine Oil Temperature).
/// Critical constraint: Gracefully maps "NO DATA", "?", or NRC 0x7F22 to Unsupported without crashing.
pub fn parse_mode22_gm_oil_temp(raw_response: &str) -> Result<ParameterValue, ObdError> {
    let clean = sanitize_response(raw_response);

    // Common ELM327 rejection outputs when a PID is unsupported
    if clean.contains("NODATA")
        || clean.contains("?")
        || clean.contains("7F22")
        || clean.contains("CANERROR")
        || clean.is_empty()
    {
        return Ok(ParameterValue::Unsupported);
    }

    // Expected positive response format: 62 11 54 XX
    if let Some(pos) = clean.find("621154") {
        let bytes_hex = &clean[pos + 6..];
        if bytes_hex.len() >= 2 {
            let raw = u32::from_str_radix(&bytes_hex[0..2], 16)
                .map_err(|e| ObdError::ParseError(e.to_string()))?;
            let temp = (raw as f32) - 40.0;
            return Ok(ParameterValue::TemperatureCelsius(temp));
        }
    }

    Ok(ParameterValue::Unsupported)
}

/// Parses SAE J1979 Mode 03 response into standard OBD2 DTCs (e.g., P0008, P0300).
pub fn parse_mode03_dtcs(raw_response: &str) -> Result<Vec<DtcRecord>, ObdError> {
    let clean = sanitize_response(raw_response);
    if clean.contains("NODATA") {
        return Ok(vec![]);
    }

    let pos = match clean.find("43") {
        Some(p) => p,
        None => return Ok(vec![]),
    };

    let dtc_hex_stream = &clean[pos + 2..];
    let mut dtcs = Vec::new();

    for chunk in dtc_hex_stream.as_bytes().as_chunks::<4>().0 {
        let pair_str = std::str::from_utf8(chunk).unwrap_or("");
        if pair_str == "0000" {
            continue; // Padding
        }

        if let Ok(val) = u16::from_str_radix(pair_str, 16) {
            let prefix = match (val >> 14) & 0x03 {
                0 => 'P',
                1 => 'C',
                2 => 'B',
                3 => 'U',
                _ => 'P',
            };
            let code = format!("{}{:04X}", prefix, val & 0x3FFF);
            dtcs.push(DtcRecord {
                code,
                description: "Standard SAE Emission Trouble Code".into(),
                status: DtcStatus::Confirmed,
                ecu_source: Some("GM ECM".into()),
            });
        }
    }

    Ok(dtcs)
}

/// Verifies Mode 04 DTC Clear confirmation.
pub fn verify_mode04_clear(raw_response: &str) -> Result<(), ObdError> {
    let clean = sanitize_response(raw_response);
    if clean.contains("44") || clean.contains("OK") {
        Ok(())
    } else {
        Err(ObdError::Elm327("Failed to clear fault codes".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elm_rpm() {
        // "41 0C 1A F8" -> ((0x1A * 256) + 0xF8) / 4 = (6656 + 248) / 4 = 1726 RPM
        let resp = "41 0C 1A F8\r\r>";
        let val = parse_mode01_rpm(resp).unwrap();
        assert_eq!(val, ParameterValue::Rpm(1726));
    }

    #[test]
    fn test_elm_speed() {
        // "41 0D 32" -> 0x32 = 50 km/h
        let resp = "41 0D 32\r>";
        let val = parse_mode01_speed(resp).unwrap();
        assert_eq!(val, ParameterValue::SpeedKmh(50));
    }

    #[test]
    fn test_elm_coolant() {
        // "41 05 7B" -> 0x7B = 123 -> 123 - 40 = 83°C
        let resp = "41 05 7B\r>";
        let val = parse_mode01_coolant_temp(resp).unwrap();
        assert_eq!(val, ParameterValue::TemperatureCelsius(83.0));
    }

    #[test]
    fn test_gmc_oil_temp_unsupported_no_data() {
        // When vehicle responds NO DATA for GM Mode 22
        let resp = "NO DATA\r\r>";
        let val = parse_mode22_gm_oil_temp(resp).unwrap();
        assert_eq!(val, ParameterValue::Unsupported);
    }

    #[test]
    fn test_gmc_oil_temp_unsupported_nrc() {
        // When vehicle responds with 7F 22 11 (Service not supported)
        let resp = "7F 22 11\r>";
        let val = parse_mode22_gm_oil_temp(resp).unwrap();
        assert_eq!(val, ParameterValue::Unsupported);
    }

    #[test]
    fn test_gmc_oil_temp_supported() {
        // "62 11 54 84" -> 0x84 = 132 -> 132 - 40 = 92°C
        let resp = "62 11 54 84\r>";
        let val = parse_mode22_gm_oil_temp(resp).unwrap();
        assert_eq!(val, ParameterValue::TemperatureCelsius(92.0));
    }

    #[test]
    fn test_mode03_dtcs() {
        // "43 00 08 03 00 00 00" -> P0008, P0300
        let resp = "43 00 08 03 00 00 00\r>";
        let dtcs = parse_mode03_dtcs(resp).unwrap();
        assert_eq!(dtcs.len(), 2);
        assert_eq!(dtcs[0].code, "P0008");
        assert_eq!(dtcs[1].code, "P0300");
    }
}

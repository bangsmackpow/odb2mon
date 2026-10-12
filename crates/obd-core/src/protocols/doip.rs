use crate::error::ObdError;

pub const DOIP_PROTOCOL_VERSION: u8 = 0x02;
pub const DOIP_INVERSE_VERSION: u8 = 0xFD;

pub const DOIP_ROUTING_ACTIVATION_REQ: u16 = 0x0005;
pub const DOIP_ROUTING_ACTIVATION_RESP: u16 = 0x0006;
pub const DOIP_DIAGNOSTIC_MESSAGE: u16 = 0x8001;
pub const DOIP_DIAGNOSTIC_POSITIVE_ACK: u16 = 0x8002;
pub const DOIP_DIAGNOSTIC_NEGATIVE_ACK: u16 = 0x8003;

/// Builds an ISO 13400-2 DoIP message frame with the standard 8-byte header.
pub fn build_doip_header(payload_type: u16, payload_len: u32) -> [u8; 8] {
    let mut header = [0u8; 8];
    header[0] = DOIP_PROTOCOL_VERSION;
    header[1] = DOIP_INVERSE_VERSION;
    header[2..4].copy_from_slice(&payload_type.to_be_bytes());
    header[4..8].copy_from_slice(&payload_len.to_be_bytes());
    header
}

/// Builds a DoIP Routing Activation Request (Payload Type 0x0005).
/// Activates diagnostic routing from tester address (typically 0x0E00 or 0x0E80).
pub fn build_routing_activation_request(tester_address: u16) -> Vec<u8> {
    let mut payload = Vec::with_capacity(7);
    payload.extend_from_slice(&tester_address.to_be_bytes());
    payload.push(0x00); // Activation Type: 0x00 (Default)
    payload.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // 4 reserved bytes

    let header = build_doip_header(DOIP_ROUTING_ACTIVATION_REQ, payload.len() as u32);
    let mut msg = Vec::with_capacity(8 + payload.len());
    msg.extend_from_slice(&header);
    msg.extend_from_slice(&payload);
    msg
}

/// Verifies Routing Activation Response (Payload Type 0x0006).
/// Expected format:
/// Bytes 0-7: DoIP header
/// Bytes 8-9: Logical address of tester
/// Bytes 10-11: Logical address of DoIP entity
/// Byte 12: Routing activation response code (0x10 = Successfully activated)
pub fn verify_routing_activation_ack(frame: &[u8]) -> Result<(), ObdError> {
    if frame.len() < 13 {
        return Err(ObdError::Protocol("DoIP Routing Activation response too short".into()));
    }

    let payload_type = u16::from_be_bytes([frame[2], frame[3]]);
    if payload_type != DOIP_ROUTING_ACTIVATION_RESP {
        return Err(ObdError::Protocol(format!(
            "Unexpected DoIP payload type: {payload_type:#06X}, expected 0x0006"
        )));
    }

    let code = frame[12];
    match code {
        0x10 => Ok(()), // Routing successfully activated
        0x00 => Err(ObdError::ConnectionFailed("DoIP Routing: Unknown source address".into())),
        0x01 => Err(ObdError::ConnectionFailed("DoIP Routing: All sockets registered".into())),
        0x02 => Err(ObdError::ConnectionFailed("DoIP Routing: Source address mismatch".into())),
        0x03 => Err(ObdError::ConnectionFailed("DoIP Routing: Source address already active".into())),
        0x04 => Err(ObdError::ConnectionFailed("DoIP Routing: Missing authentication".into())),
        0x05 => Err(ObdError::ConnectionFailed("DoIP Routing: Missing confirmation".into())),
        0x06 => Err(ObdError::ConnectionFailed("DoIP Routing: Unsupported activation type".into())),
        _ => Err(ObdError::ConnectionFailed(format!("DoIP Routing failed with code {code:#04X}"))),
    }
}

/// Packs a UDS diagnostic request into an ISO 13400-2 Diagnostic Message (0x8001).
pub fn build_diagnostic_message(source_address: u16, target_address: u16, uds_payload: &[u8]) -> Vec<u8> {
    let payload_len = 4 + uds_payload.len();
    let header = build_doip_header(DOIP_DIAGNOSTIC_MESSAGE, payload_len as u32);

    let mut msg = Vec::with_capacity(8 + payload_len);
    msg.extend_from_slice(&header);
    msg.extend_from_slice(&source_address.to_be_bytes());
    msg.extend_from_slice(&target_address.to_be_bytes());
    msg.extend_from_slice(uds_payload);
    msg
}

/// Extracts the raw UDS payload from a DoIP Diagnostic Message (0x8001).
/// Skips the 4-byte source and target addresses.
pub fn extract_uds_payload(doip_payload: &[u8]) -> Result<Vec<u8>, ObdError> {
    if doip_payload.len() < 4 {
        return Err(ObdError::ParseError("DoIP diagnostic payload too short".into()));
    }
    // Skip 2 bytes source address, 2 bytes target address
    Ok(doip_payload[4..].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routing_activation_builder() {
        let req = build_routing_activation_request(0x0E00);
        assert_eq!(req.len(), 15);
        assert_eq!(req[0], DOIP_PROTOCOL_VERSION);
        assert_eq!(req[1], DOIP_INVERSE_VERSION);
        assert_eq!(req[2..4], [0x00, 0x05]);
        assert_eq!(req[4..8], [0x00, 0x00, 0x00, 0x07]); // 7 bytes payload
        assert_eq!(req[8..10], [0x0E, 0x00]);
    }

    #[test]
    fn test_diagnostic_message_builder() {
        let uds = [0x22, 0xD0, 0x01];
        let msg = build_diagnostic_message(0x0E00, 0x0012, &uds);
        assert_eq!(msg.len(), 8 + 4 + 3);
        assert_eq!(msg[2..4], [0x80, 0x01]);
        assert_eq!(msg[8..10], [0x0E, 0x00]);
        assert_eq!(msg[10..12], [0x00, 0x12]);
        assert_eq!(&msg[12..], &uds);
    }
}

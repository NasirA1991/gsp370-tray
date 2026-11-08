#![allow(dead_code)]

pub const VENDOR_ID: u16 = 0x1395;
pub const PRODUCT_ID: u16 = 0x009A;
pub const BAUD_RATE: u32 = 921_600;

pub const OP_BATTERY: u8 = 0x9C;
pub const OP_SIDETONE_ON: u8 = 0xA0;
pub const OP_SIDETONE_OFF: u8 = 0xA1;
pub const OP_SIDETONE_SET: u8 = 0xA2;

pub const STATUS_OK: u8 = 0x00;
pub const STATUS_HEADSET_OFF: u8 = 0x0C;

pub const SIDETONE_DEVICE_MIN: u8 = 6;
pub const SIDETONE_DEVICE_MAX: u8 = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    pub status: u8,
    pub values: Vec<u8>,
}

pub fn make_command(opcode: u8, params: &[u8]) -> Vec<u8> {
    assert!(params.len() <= u8::MAX as usize);
    let mut packet = Vec::with_capacity(4 + params.len());
    packet.extend_from_slice(&[0x01, opcode, 0xFC, params.len() as u8]);
    packet.extend_from_slice(params);
    packet
}

/// Parse `04 0e <len> 01 <op> fc <status> <values...>`.
///
/// Returns None for an incomplete or absent matching frame.  Unrelated
/// asynchronous events are skipped.
pub fn parse_reply(received: &[u8], opcode: u8) -> Option<Reply> {
    let mut i = 0;
    while i + 6 < received.len() {
        if received[i] != 0x04 || received[i + 1] != 0x0E {
            i += 1;
            continue;
        }
        let payload_len = received[i + 2] as usize;
        if payload_len < 4
            || received[i + 3] != 0x01
            || received[i + 4] != opcode
            || received[i + 5] != 0xFC
        {
            i += 1;
            continue;
        }
        let end = i + 3 + payload_len;
        if end > received.len() {
            // This candidate is incomplete.  Do not let it hide a complete
            // matching frame that follows it (for example after serial noise
            // or a dropped frame).
            i += 1;
            continue;
        }
        return Some(Reply {
            status: received[i + 6],
            values: received[i + 7..end].to_vec(),
        });
    }
    None
}

/// 0 % is off; 1..100 map to the 11 audible firmware steps 6..16.
pub fn sidetone_level(percent: u8) -> u8 {
    let percent = percent.min(100);
    if percent == 0 {
        return 0;
    }
    let span = (SIDETONE_DEVICE_MAX - SIDETONE_DEVICE_MIN) as u16;
    SIDETONE_DEVICE_MIN + ((percent as u16 - 1) * span / 99) as u8
}

pub fn opcode_name(opcode: u8) -> &'static str {
    match opcode {
        OP_BATTERY => "battery",
        OP_SIDETONE_ON => "sidetone-on",
        OP_SIDETONE_OFF => "sidetone-off",
        OP_SIDETONE_SET => "sidetone-set",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands() {
        assert_eq!(make_command(OP_BATTERY, &[]), vec![0x01, 0x9C, 0xFC, 0x00]);
        assert_eq!(
            make_command(OP_SIDETONE_SET, &[0x08]),
            vec![0x01, 0xA2, 0xFC, 0x01, 0x08]
        );
    }

    #[test]
    fn reply_after_event() {
        let rx = [
            0x04, 0xE9, 0x00, 0x04, 0x0E, 0x05, 0x01, 0x9C, 0xFC, 0x00, 0x57,
        ];
        let reply = parse_reply(&rx, OP_BATTERY).unwrap();
        assert_eq!(reply.status, 0);
        assert_eq!(reply.values, vec![0x57]);
        assert!(parse_reply(&rx[..9], OP_BATTERY).is_none());
        assert!(parse_reply(&rx, OP_SIDETONE_SET).is_none());
    }

    #[test]
    fn status_only_and_off() {
        let off = [0x04, 0x0E, 0x04, 0x01, 0x9C, 0xFC, 0x0C];
        let reply = parse_reply(&off, OP_BATTERY).unwrap();
        assert_eq!(reply.status, STATUS_HEADSET_OFF);
        assert!(reply.values.is_empty());

        let ok = [0x04, 0x0E, 0x04, 0x01, 0xA2, 0xFC, 0x00];
        assert!(parse_reply(&ok, OP_SIDETONE_SET).is_some());
    }

    #[test]
    fn sidetone_scale() {
        assert_eq!(sidetone_level(0), 0);
        assert_eq!(sidetone_level(1), 6);
        assert_eq!(sidetone_level(10), 6);
        assert_eq!(sidetone_level(20), 7);
        assert_eq!(sidetone_level(50), 10);
        assert_eq!(sidetone_level(100), 16);
        assert_eq!(sidetone_level(101), 16);
    }

    #[test]
    fn resyncs_past_an_incomplete_frame() {
        let truncated_then_valid = [
            0x04, 0x0E, 0x20, 0x01, 0x9C, 0xFC, // incomplete candidate
            0x04, 0x0E, 0x05, 0x01, 0x9C, 0xFC, 0x00, 0x57,
        ];
        let reply = parse_reply(&truncated_then_valid, OP_BATTERY).unwrap();
        assert_eq!(reply.values, vec![0x57]);
    }

    #[test]
    fn malformed() {
        let short = [0x04, 0x0E, 0x02, 0x01, 0x9C, 0xFC, 0x00];
        assert!(parse_reply(&short, OP_BATTERY).is_none());
        let wrong_report = [0x01, 0x0E, 0x05, 0x01, 0x9C, 0xFC, 0x00, 0x57];
        assert!(parse_reply(&wrong_report, OP_BATTERY).is_none());
    }
}

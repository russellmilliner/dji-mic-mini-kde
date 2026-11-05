use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SOF: u8 = 0x55;
pub const VERSION: u8 = 0x04;

pub const CMD_SET_GENERAL: u8 = 0x00;
pub const CMD_ID_GET_VERSION: u8 = 0x01;

pub const CMD_SET_AUX: u8 = 0x04;
pub const CMD_ID_AUX_74: u8 = 0x74;

pub const CMD_SET_MIC: u8 = 0x5B;
pub const CMD_ID_SET_PARAM: u8 = 0x01;
pub const CMD_ID_STATUS_PUSH: u8 = 0x03;
pub const CMD_ID_HEARTBEAT_PUSH: u8 = 0x05;

pub const CRC8_INIT: u8 = 0x77;

const CRC8_TABLE: [u8; 256] = [
    0x00, 0x5E, 0xBC, 0xE2, 0x61, 0x3F, 0xDD, 0x83, 0xC2, 0x9C, 0x7E, 0x20, 0xA3, 0xFD, 0x1F, 0x41,
    0x9D, 0xC3, 0x21, 0x7F, 0xFC, 0xA2, 0x40, 0x1E, 0x5F, 0x01, 0xE3, 0xBD, 0x3E, 0x60, 0x82, 0xDC,
    0x23, 0x7D, 0x9F, 0xC1, 0x42, 0x1C, 0xFE, 0xA0, 0xE1, 0xBF, 0x5D, 0x03, 0x80, 0xDE, 0x3C, 0x62,
    0xBE, 0xE0, 0x02, 0x5C, 0xDF, 0x81, 0x63, 0x3D, 0x7C, 0x22, 0xC0, 0x9E, 0x1D, 0x43, 0xA1, 0xFF,
    0x46, 0x18, 0xFA, 0xA4, 0x27, 0x79, 0x9B, 0xC5, 0x84, 0xDA, 0x38, 0x66, 0xE5, 0xBB, 0x59, 0x07,
    0xDB, 0x85, 0x67, 0x39, 0xBA, 0xE4, 0x06, 0x58, 0x19, 0x47, 0xA5, 0xFB, 0x78, 0x26, 0xC4, 0x9A,
    0x65, 0x3B, 0xD9, 0x87, 0x04, 0x5A, 0xB8, 0xE6, 0xA7, 0xF9, 0x1B, 0x45, 0xC6, 0x98, 0x7A, 0x24,
    0xF8, 0xA6, 0x44, 0x1A, 0x99, 0xC7, 0x25, 0x7B, 0x3A, 0x64, 0x86, 0xD8, 0x5B, 0x05, 0xE7, 0xB9,
    0x8C, 0xD2, 0x30, 0x6E, 0xED, 0xB3, 0x51, 0x0F, 0x4E, 0x10, 0xF2, 0xAC, 0x2F, 0x71, 0x93, 0xCD,
    0x11, 0x4F, 0xAD, 0xF3, 0x70, 0x2E, 0xCC, 0x92, 0xD3, 0x8D, 0x6F, 0x31, 0xB2, 0xEC, 0x0E, 0x50,
    0xAF, 0xF1, 0x13, 0x4D, 0xCE, 0x90, 0x72, 0x2C, 0x6D, 0x33, 0xD1, 0x8F, 0x0C, 0x52, 0xB0, 0xEE,
    0x32, 0x6C, 0x8E, 0xD0, 0x53, 0x0D, 0xEF, 0xB1, 0xF0, 0xAE, 0x4C, 0x12, 0x91, 0xCF, 0x2D, 0x73,
    0xCA, 0x94, 0x76, 0x28, 0xAB, 0xF5, 0x17, 0x49, 0x08, 0x56, 0xB4, 0xEA, 0x69, 0x37, 0xD5, 0x8B,
    0x57, 0x09, 0xEB, 0xB5, 0x36, 0x68, 0x8A, 0xD4, 0x95, 0xCB, 0x29, 0x77, 0xF4, 0xAA, 0x48, 0x16,
    0xE9, 0xB7, 0x55, 0x0B, 0x88, 0xD6, 0x34, 0x6A, 0x2B, 0x75, 0x97, 0xC9, 0x4A, 0x14, 0xF6, 0xA8,
    0x74, 0x2A, 0xC8, 0x96, 0x15, 0x4B, 0xA9, 0xF7, 0xB6, 0xE8, 0x0A, 0x54, 0xD7, 0x89, 0x6B, 0x35,
];

const CRC16_TABLE: [u16; 256] = [
    0x0000, 0x1189, 0x2312, 0x329B, 0x4624, 0x57AD, 0x6536, 0x74BF, 0x8C48, 0x9DC1, 0xAF5A, 0xBED3,
    0xCA6C, 0xDBE5, 0xE97E, 0xF8F7, 0x1081, 0x0108, 0x3393, 0x221A, 0x56A5, 0x472C, 0x75B7, 0x643E,
    0x9CC9, 0x8D40, 0xBFDB, 0xAE52, 0xDAED, 0xCB64, 0xF9FF, 0xE876, 0x2102, 0x308B, 0x0210, 0x1399,
    0x6726, 0x76AF, 0x4434, 0x55BD, 0xAD4A, 0xBCC3, 0x8E58, 0x9FD1, 0xEB6E, 0xFAE7, 0xC87C, 0xD9F5,
    0x3183, 0x200A, 0x1291, 0x0318, 0x77A7, 0x662E, 0x54B5, 0x453C, 0xBDCB, 0xAC42, 0x9ED9, 0x8F50,
    0xFBEF, 0xEA66, 0xD8FD, 0xC974, 0x4204, 0x538D, 0x6116, 0x709F, 0x0420, 0x15A9, 0x2732, 0x36BB,
    0xCE4C, 0xDFC5, 0xED5E, 0xFCD7, 0x8868, 0x99E1, 0xAB7A, 0xBAF3, 0x5285, 0x430C, 0x7197, 0x601E,
    0x14A1, 0x0528, 0x37B3, 0x263A, 0xDECD, 0xCF44, 0xFDDF, 0xEC56, 0x98E9, 0x8960, 0xBBFB, 0xAA72,
    0x6306, 0x728F, 0x4014, 0x519D, 0x2522, 0x34AB, 0x0630, 0x17B9, 0xEF4E, 0xFEC7, 0xCC5C, 0xDDD5,
    0xA96A, 0xB8E3, 0x8A78, 0x9BF1, 0x7387, 0x620E, 0x5095, 0x411C, 0x35A3, 0x242A, 0x16B1, 0x0738,
    0xFFCF, 0xEE46, 0xDCDD, 0xCD54, 0xB9EB, 0xA862, 0x9AF9, 0x8B70, 0x8408, 0x9581, 0xA71A, 0xB693,
    0xC22C, 0xD3A5, 0xE13E, 0xF0B7, 0x0840, 0x19C9, 0x2B52, 0x3ADB, 0x4E64, 0x5FED, 0x6D76, 0x7CFF,
    0x9489, 0x8500, 0xB79B, 0xA612, 0xD2AD, 0xC324, 0xF1BF, 0xE036, 0x18C1, 0x0948, 0x3BD3, 0x2A5A,
    0x5EE5, 0x4F6C, 0x7DF7, 0x6C7E, 0xA50A, 0xB483, 0x8618, 0x9791, 0xE32E, 0xF2A7, 0xC03C, 0xD1B5,
    0x2942, 0x38CB, 0x0A50, 0x1BD9, 0x6F66, 0x7EEF, 0x4C74, 0x5DFD, 0xB58B, 0xA402, 0x9699, 0x8710,
    0xF3AF, 0xE226, 0xD0BD, 0xC134, 0x39C3, 0x284A, 0x1AD1, 0x0B58, 0x7FE7, 0x6E6E, 0x5CF5, 0x4D7C,
    0xC60C, 0xD785, 0xE51E, 0xF497, 0x8028, 0x91A1, 0xA33A, 0xB2B3, 0x4A44, 0x5BCD, 0x6956, 0x78DF,
    0x0C60, 0x1DE9, 0x2F72, 0x3EFB, 0xD68D, 0xC704, 0xF59F, 0xE416, 0x90A9, 0x8120, 0xB3BB, 0xA232,
    0x5AC5, 0x4B4C, 0x79D7, 0x685E, 0x1CE1, 0x0D68, 0x3FF3, 0x2E7A, 0xE70E, 0xF687, 0xC41C, 0xD595,
    0xA12A, 0xB0A3, 0x8238, 0x93B1, 0x6B46, 0x7ACF, 0x4854, 0x59DD, 0x2D62, 0x3CEB, 0x0E70, 0x1FF9,
    0xF78F, 0xE606, 0xD49D, 0xC514, 0xB1AB, 0xA022, 0x92B9, 0x8330, 0x7BC7, 0x6A4E, 0x58D5, 0x495C,
    0x3DE3, 0x2C6A, 0x1EF1, 0x0F78,
];

pub const CRC16_V1_INIT: u16 = 0x0000;
pub const CRC16_V1_RESIDUE: u16 = 0xBB01;
pub const CRC16_V2_INIT: u16 = 0x3692;
pub const CRC16_V2_RESIDUE: u16 = 0x0000;

pub fn crc8(data: &[u8], init: u8) -> u8 {
    let mut crc = init;
    for &b in data {
        crc = CRC8_TABLE[(crc ^ b) as usize];
    }
    crc
}

pub fn crc16_step(crc: u16, byte: u8) -> u16 {
    (crc >> 8) ^ CRC16_TABLE[((crc ^ (byte as u16)) & 0xFF) as usize]
}

pub fn crc16(data: &[u8], init: u16) -> u16 {
    let mut crc = init;
    for &b in data {
        crc = crc16_step(crc, b);
    }
    crc & 0xFFFF
}

pub fn calc_frame_crc16_bytes(frame_without_crc: &[u8], dialect: &str) -> [u8; 2] {
    let (init, residue) = if dialect == "v1" {
        (CRC16_V1_INIT, CRC16_V1_RESIDUE)
    } else {
        (CRC16_V2_INIT, CRC16_V2_RESIDUE)
    };
    let v = crc16(frame_without_crc, init);
    for b0 in 0u16..256 {
        let v1 = crc16_step(v, b0 as u8);
        let target = residue ^ (v1 >> 8);
        if let Some(idx) = CRC16_TABLE.iter().position(|&x| x == target) {
            let b1 = ((v1 ^ (idx as u16)) & 0xFF) as u8;
            return [b0 as u8, b1];
        }
    }
    v.to_le_bytes()
}

pub fn battery_code_to_percent(code: Option<i32>) -> Option<i32> {
    match code {
        None => None,
        Some(1) => Some(100),
        Some(2) => Some(85),
        Some(3) => Some(70),
        Some(4) => Some(55),
        Some(5) => Some(40),
        Some(6) => Some(25),
        Some(7) => Some(5),
        Some(c) if c > 7 && c <= 100 => Some(c),
        Some(c) => Some(c.clamp(0, 100)),
    }
}

pub fn gain_step_to_db(step_code: i32) -> Option<i32> {
    match step_code {
        1 => Some(-12),
        2 => Some(-6),
        3 => Some(0),
        4 => Some(6),
        5 => Some(12),
        _ => None,
    }
}

pub fn parse_ascii_string(data: &[u8]) -> String {
    let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
    data[..end]
        .iter()
        .filter(|&&b| (0x20..0x7F).contains(&b))
        .map(|&b| b as char)
        .collect::<String>()
        .trim()
        .to_string()
}

pub fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RxInfo {
    pub serial: Option<String>,
    pub firmware: Option<String>,
    pub hardware_model: Option<String>,
    pub hardware_revision: Option<String>,
    pub product_name: Option<String>,
    pub bluetooth_id: Option<String>,
    pub battery: Option<i32>,
    pub battery_percent: Option<i32>,
    pub gain_step_code: Option<i32>,
    pub raw_flags0: Option<u8>,
    pub raw_flags1: Option<u8>,
    pub raw_plug_free_byte: Option<u8>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TxInfo {
    pub unit: i32,
    pub connected: bool,
    pub level: Option<i32>,
    pub serial: Option<String>,
    pub firmware: Option<String>,
    pub product_name: Option<String>,
    pub bluetooth_id: Option<String>,
    pub voice_tone: Option<String>,
    pub charging: bool,
    pub battery: Option<i32>,
    pub battery_percent: Option<i32>,
    pub nc_enabled: Option<bool>,
    pub nc_mode: Option<String>,
    pub low_cut: Option<bool>,
    pub auto_off_15m: Option<bool>,
    pub nc_button_enabled: Option<bool>,
    pub leds_enabled: Option<bool>,
    pub raw_state_byte: Option<u8>,
    pub raw_flags_byte: Option<u8>,
    pub raw_led_byte: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStatus {
    pub connected: bool,
    pub dialect: String,
    pub gain_dial_db: Option<i32>,
    pub nc_enabled: Option<bool>,
    pub rx: Option<RxInfo>,
    pub tx: Vec<Option<TxInfo>>,
    pub settings: BTreeMap<String, String>,
    pub paired_tx_count_hid: Option<i32>,
    pub active_tx_index_hid: Option<i32>,
    pub last_sequence: u16,
    pub heartbeat_count: u64,
    pub last_heartbeat_hex: Option<String>,
    pub aux_5b05_hex: Option<String>,
    pub aux_0474_hex: Option<String>,
}

impl Default for DeviceStatus {
    fn default() -> Self {
        Self {
            connected: false,
            dialect: "v1".to_string(),
            gain_dial_db: None,
            nc_enabled: None,
            rx: None,
            tx: vec![None, None],
            settings: BTreeMap::new(),
            paired_tx_count_hid: None,
            active_tx_index_hid: None,
            last_sequence: 0,
            heartbeat_count: 0,
            last_heartbeat_hex: None,
            aux_5b05_hex: None,
            aux_0474_hex: None,
        }
    }
}

impl DeviceStatus {
    pub fn tx_count(&self) -> i32 {
        self.tx
            .iter()
            .filter(|t| t.as_ref().map(|x| x.connected).unwrap_or(false))
            .count() as i32
    }
}

#[derive(Debug, Clone)]
pub struct DumlFrame {
    pub length: usize,
    pub version: u8,
    pub sender: u8,
    pub receiver: u8,
    pub seq: u16,
    pub pack_type: u8,
    pub cmd_set: u8,
    pub cmd_id: u8,
    pub payload: Vec<u8>,
    pub raw: Vec<u8>,
    pub dialect: String,
}

pub fn detect_dialect(raw: &[u8]) -> Option<&'static str> {
    if raw.len() < 12 {
        return None;
    }
    if raw[8] == 0x00 && raw[9] == 0x5B && raw[10] == 0x03 {
        if raw[11] == 0x00 {
            return Some("v1");
        } else if raw[11] == 0x03 {
            return Some("v2");
        }
    }
    None
}

pub fn decode_frame(raw: &[u8]) -> Option<DumlFrame> {
    if raw.len() < 13 || raw[0] != SOF || (raw[2] & 0xFC) != VERSION {
        return None;
    }
    let length = (raw[1] as usize) | (((raw[2] & 0x03) as usize) << 8);
    if length != raw.len() || !(13..=256).contains(&length) {
        return None;
    }
    if crc8(&raw[..4], CRC8_INIT) != 0 {
        return None;
    }

    let dialect = if let Some(d) = detect_dialect(raw) {
        d
    } else if crc16(raw, CRC16_V1_INIT) == CRC16_V1_RESIDUE {
        "v1"
    } else if crc16(raw, CRC16_V2_INIT) == CRC16_V2_RESIDUE {
        "v2"
    } else {
        "v1"
    };

    let sender = raw[4];
    let receiver = raw[5];
    let seq = u16::from_le_bytes([raw[6], raw[7]]);
    let pack_type = raw[8];
    let cmd_set = raw[9];
    let cmd_id = raw[10];
    let payload = raw[11..raw.len() - 2].to_vec();

    Some(DumlFrame {
        length,
        version: 1,
        sender,
        receiver,
        seq,
        pack_type,
        cmd_set,
        cmd_id,
        payload,
        raw: raw.to_vec(),
        dialect: dialect.to_string(),
    })
}

pub fn take_frames(buffer: &mut Vec<u8>) -> Vec<DumlFrame> {
    let mut frames = Vec::new();
    loop {
        if buffer.len() < 4 {
            break;
        }
        let mut start: Option<usize> = None;
        for i in 0..=(buffer.len() - 4) {
            if buffer[i] == SOF && (buffer[i + 2] & 0xFC) == VERSION {
                let flen = (buffer[i + 1] as usize) | (((buffer[i + 2] & 0x03) as usize) << 8);
                if (13..=256).contains(&flen) && crc8(&buffer[i..i + 4], CRC8_INIT) == 0 {
                    start = Some(i);
                    break;
                }
            }
        }

        let Some(sof) = start else {
            let keep = buffer.len().min(3);
            let drain_to = buffer.len() - keep;
            buffer.drain(..drain_to);
            break;
        };

        if sof > 0 {
            buffer.drain(..sof);
        }
        if buffer.len() < 4 {
            break;
        }

        let frame_len = (buffer[1] as usize) | (((buffer[2] & 0x03) as usize) << 8);
        if buffer.len() < frame_len {
            break;
        }

        let raw_frame: Vec<u8> = buffer.drain(..frame_len).collect();
        if let Some(df) = decode_frame(&raw_frame) {
            frames.push(df);
        }
    }
    frames
}

pub fn decode_v1_heartbeat(status: &mut DeviceStatus, raw: &[u8]) {
    let header_len = 14usize;
    let rx_len = 22usize;
    if raw.len() < header_len + rx_len {
        return;
    }

    status.dialect = "v1".to_string();
    status.connected = true;
    status.last_sequence = u16::from_le_bytes([raw[6], raw[7]]);
    status.heartbeat_count += 1;
    status.last_heartbeat_hex = Some(hex_encode(raw));

    let mut off = header_len;
    let mut tx_list: Vec<Option<TxInfo>> = vec![None, None];
    let mut state_byte: u8 = 0;
    let mut flags_byte: u8 = 0;
    let mut led_off: Option<bool> = None;

    for slot_idx in 0..2 {
        if off + 1 >= raw.len() {
            break;
        }
        let s_byte = raw[off];
        let f_byte = raw[off + 1];
        if (f_byte & 0x20) != 0 {
            if off + 23 > raw.len() {
                break;
            }
            state_byte = s_byte;
            flags_byte = f_byte;
            let level = raw[off + 2] as i32;
            let raw_led = raw[off + 3];
            let this_led_off = (raw_led & 0x80) != 0;
            led_off = Some(this_led_off);

            let fw_bytes = &raw[off + 4..off + 8];
            let fw_str = format!(
                "{:02}.{:02}.{:02}.{:02}",
                fw_bytes[0], fw_bytes[1], fw_bytes[2], fw_bytes[3]
            );
            let str_len = (raw[off + 8] as usize).min(14);
            let serial = parse_ascii_string(&raw[off + 9..off + 9 + str_len]);

            let bat_code = ((f_byte >> 2) & 0x07) as i32;
            let bat_pct = battery_code_to_percent(Some(bat_code));
            let nc_active = (f_byte & 0x01) != 0;
            let nc_strength = if (s_byte & 0x20) != 0 {
                "strong"
            } else {
                "basic"
            }
            .to_string();
            let low_cut_on = (s_byte & 0x04) != 0;
            let tx_auto_off = (s_byte & 0x10) != 0;
            let nc_btn_on = (s_byte & 0x80) != 0;

            let prev_tx = status.tx.get(slot_idx).and_then(|t| t.as_ref());
            let prod_name = prev_tx
                .and_then(|p| p.product_name.clone())
                .unwrap_or_else(|| "DJI Mic Mini TX".to_string());
            let bt_id = prev_tx.and_then(|p| p.bluetooth_id.clone());
            let v_tone = prev_tx
                .and_then(|p| p.voice_tone.clone())
                .unwrap_or_else(|| "standard".to_string());

            tx_list[slot_idx] = Some(TxInfo {
                unit: (slot_idx + 1) as i32,
                connected: true,
                level: Some(level),
                serial: if serial.is_empty() { None } else { Some(serial) },
                firmware: Some(fw_str),
                product_name: Some(prod_name),
                bluetooth_id: bt_id,
                voice_tone: Some(v_tone),
                charging: false,
                battery: Some(bat_code),
                battery_percent: bat_pct,
                nc_enabled: Some(nc_active),
                nc_mode: Some(nc_strength),
                low_cut: Some(low_cut_on),
                auto_off_15m: Some(tx_auto_off),
                nc_button_enabled: Some(nc_btn_on),
                leds_enabled: Some(!this_led_off),
                raw_state_byte: Some(s_byte),
                raw_flags_byte: Some(f_byte),
                raw_led_byte: Some(raw_led),
            });
            off += 23;
        } else {
            if off + 9 > raw.len() {
                break;
            }
            if state_byte == 0 && flags_byte == 0 {
                state_byte = s_byte;
                flags_byte = f_byte;
            }
            off += 9;
        }
    }

    status.tx = tx_list;
    status.nc_enabled = Some((flags_byte & 0x01) != 0);

    if off + rx_len <= raw.len() {
        let rx_flags0 = raw[off];
        let rx_flags1 = raw[off + 1];
        let fw_bytes = &raw[off + 2..off + 6];
        let fw_str = format!(
            "{:02}.{:02}.{:02}.{:02}",
            fw_bytes[0], fw_bytes[1], fw_bytes[2], fw_bytes[3]
        );
        let str_len = (raw[off + 6] as usize).min(14);
        let serial = parse_ascii_string(&raw[off + 7..off + 7 + str_len]);
        let plug_free_byte = raw[off + 21];
        let plug_free = plug_free_byte == 0x01;

        let gain_code = ((rx_flags0 >> 4) & 0x07) as i32;
        if let Some(db) = gain_step_to_db(gain_code) {
            status.gain_dial_db = Some(db);
        }

        let prev_rx = status.rx.clone();
        status.rx = Some(RxInfo {
            serial: if serial.is_empty() { None } else { Some(serial) },
            firmware: Some(fw_str),
            hardware_model: prev_rx
                .as_ref()
                .and_then(|r| r.hardware_model.clone())
                .unwrap_or_else(|| "DJI MIC MINI RX".to_string())
                .into(),
            hardware_revision: prev_rx.as_ref().and_then(|r| r.hardware_revision.clone()),
            product_name: prev_rx
                .as_ref()
                .and_then(|r| r.product_name.clone())
                .or_else(|| Some("DJI Mic Mini Receiver".to_string())),
            bluetooth_id: prev_rx.as_ref().and_then(|r| r.bluetooth_id.clone()),
            battery: None,
            battery_percent: None,
            gain_step_code: Some(gain_code),
            raw_flags0: Some(rx_flags0),
            raw_flags1: Some(rx_flags1),
            raw_plug_free_byte: Some(plug_free_byte),
        });

        status.settings.insert(
            "noise-cancel".to_string(),
            if (state_byte & 0x20) != 0 {
                "strong"
            } else {
                "basic"
            }
            .to_string(),
        );
        status.settings.insert(
            "noise-cancel-power".to_string(),
            if status.nc_enabled.unwrap_or(false) {
                "on"
            } else {
                "off"
            }
            .to_string(),
        );
        status.settings.insert(
            "noise-cancel-button".to_string(),
            if (state_byte & 0x80) != 0 {
                "on"
            } else {
                "off"
            }
            .to_string(),
        );
        status.settings.insert(
            "tx-auto-off-15m".to_string(),
            if (state_byte & 0x10) != 0 {
                "on"
            } else {
                "off"
            }
            .to_string(),
        );
        status.settings.insert(
            "low-cut".to_string(),
            if (state_byte & 0x04) != 0 {
                "on"
            } else {
                "off"
            }
            .to_string(),
        );
        status.settings.insert(
            "stereo".to_string(),
            if (rx_flags0 & 0x08) != 0 {
                "stereo"
            } else {
                "mono"
            }
            .to_string(),
        );
        status.settings.insert(
            "camera-power".to_string(),
            if (rx_flags0 & 0x01) != 0 {
                "on"
            } else {
                "off"
            }
            .to_string(),
        );
        status.settings.insert(
            "auto-off-15m".to_string(),
            if (rx_flags0 & 0x02) != 0 {
                "on"
            } else {
                "off"
            }
            .to_string(),
        );
        status.settings.insert(
            "safety-track".to_string(),
            if (rx_flags1 & 0x80) != 0 {
                "on"
            } else {
                "off"
            }
            .to_string(),
        );
        status.settings.insert(
            "clip-limiter".to_string(),
            if (rx_flags1 & 0x20) != 0 {
                "on"
            } else {
                "off"
            }
            .to_string(),
        );
        status.settings.insert(
            "plug-free".to_string(),
            if plug_free { "on" } else { "off" }.to_string(),
        );
        if let Some(lo) = led_off {
            status.settings.insert(
                "mic-leds".to_string(),
                if lo { "off" } else { "on" }.to_string(),
            );
        }
    }
}

pub fn decode_v2_status(status: &mut DeviceStatus, raw: &[u8]) -> bool {
    let header_len = 52usize;
    let slot_len = 32usize;
    if raw.len() < header_len + 2 {
        return false;
    }
    let remainder = raw.len() - (header_len + 2);
    if remainder % slot_len != 0 {
        return false;
    }
    let tx_slots_in_frame = (remainder / slot_len).min(2);

    let flags1 = raw[20];
    let flags2 = raw[21];
    let dial_raw = raw[22] as i8;
    status.gain_dial_db = Some(dial_raw as i32);
    let flags3 = raw[48];

    status.dialect = "v2".to_string();
    status.connected = true;
    status.last_sequence = u16::from_le_bytes([raw[6], raw[7]]);
    status.heartbeat_count += 1;
    status.last_heartbeat_hex = Some(hex_encode(raw));
    status.nc_enabled = Some((flags1 & 0x20) != 0);

    if status.rx.is_none() {
        status.rx = Some(RxInfo {
            product_name: Some("DJI Mic Mini Receiver".to_string()),
            ..Default::default()
        });
    }
    if let Some(ref mut rx) = status.rx {
        rx.raw_flags0 = Some(flags1);
        rx.raw_flags1 = Some(flags2);
        let bat = ((flags2 >> 5) & 0x07) as i32;
        rx.battery = Some(bat);
        rx.battery_percent = battery_code_to_percent(Some(bat));
    }

    let connected_mask = raw[44];
    for i in 0..2 {
        let is_conn = (connected_mask & (1 << i)) != 0;
        if is_conn {
            if status.tx[i].is_none() {
                status.tx[i] = Some(TxInfo {
                    unit: (i + 1) as i32,
                    connected: true,
                    ..Default::default()
                });
            } else if let Some(ref mut t) = status.tx[i] {
                t.connected = true;
            }
        } else {
            status.tx[i] = None;
        }
    }

    let mut tx_flags: Option<u8> = None;
    let mut low_cut_flags: Option<u8> = None;

    for pos in 0..tx_slots_in_frame {
        let slot_off = header_len + pos * slot_len;
        let unit_byte = raw[slot_off + 1] as isize;
        let idx = unit_byte - 1;
        if (0..2).contains(&idx) {
            if let Some(ref mut tx) = status.tx[idx as usize] {
                let slot_flags_6 = raw[slot_off + 6];
                let slot_flags_7 = raw[slot_off + 7];
                let slot_flags_9 = raw[slot_off + 9];

                tx_flags = Some(slot_flags_6);
                low_cut_flags = Some(slot_flags_9);

                tx.voice_tone = Some(
                    if (slot_flags_9 & 0x80) != 0 {
                        "bright"
                    } else if (slot_flags_9 & 0x40) != 0 {
                        "rich"
                    } else {
                        "standard"
                    }
                    .to_string(),
                );
                tx.charging = (slot_flags_7 & 0x02) != 0;
                let bat = ((slot_flags_7 >> 2) & 0x07) as i32;
                tx.battery = Some(bat);
                tx.battery_percent = battery_code_to_percent(Some(bat));
                tx.nc_enabled = Some((slot_flags_7 & 0x01) != 0);
                tx.leds_enabled = Some((slot_flags_6 & 0x02) == 0);
                tx.auto_off_15m = Some((slot_flags_6 & 0x10) != 0);
                tx.nc_button_enabled = Some((slot_flags_6 & 0x80) != 0);
                tx.low_cut = Some((slot_flags_9 & 0x20) != 0);
                tx.raw_state_byte = Some(slot_flags_6);
                tx.raw_flags_byte = Some(slot_flags_7);
            }
        }
    }

    status.settings.insert(
        "noise-cancel".to_string(),
        if (flags1 & 0x08) != 0 { "strong" } else { "basic" }.to_string(),
    );
    status.settings.insert(
        "noise-cancel-power".to_string(),
        if (flags1 & 0x20) != 0 { "on" } else { "off" }.to_string(),
    );
    status.settings.insert(
        "camera-power".to_string(),
        if (flags1 & 0x80) != 0 { "on" } else { "off" }.to_string(),
    );
    status.settings.insert(
        "auto-off-15m".to_string(),
        if (flags2 & 0x01) != 0 { "on" } else { "off" }.to_string(),
    );
    status.settings.insert(
        "stereo".to_string(),
        if (flags2 & 0x04) != 0 { "stereo" } else { "mono" }.to_string(),
    );
    status.settings.insert(
        "plug-free".to_string(),
        if (flags3 & 0x02) != 0 { "on" } else { "off" }.to_string(),
    );
    status.settings.insert(
        "clip-limiter".to_string(),
        if (flags3 & 0x10) != 0 { "on" } else { "off" }.to_string(),
    );
    status.settings.insert(
        "safety-track".to_string(),
        if (flags3 & 0x40) != 0 { "on" } else { "off" }.to_string(),
    );

    if let Some(tf) = tx_flags {
        status.settings.insert(
            "mic-leds".to_string(),
            if (tf & 0x02) != 0 { "off" } else { "on" }.to_string(),
        );
        status.settings.insert(
            "tx-auto-off-15m".to_string(),
            if (tf & 0x10) != 0 { "on" } else { "off" }.to_string(),
        );
        status.settings.insert(
            "noise-cancel-button".to_string(),
            if (tf & 0x80) != 0 { "on" } else { "off" }.to_string(),
        );
    }

    if let Some(lcf) = low_cut_flags {
        status.settings.insert(
            "low-cut".to_string(),
            if (lcf & 0x20) != 0 { "on" } else { "off" }.to_string(),
        );
    }

    true
}

pub fn decode_v2_records(status: &mut DeviceStatus, raw: &[u8]) {
    let records_start = 14usize;
    let record_hdr_len = 6usize;
    if raw.len() < records_start + 2 {
        return;
    }
    let end = raw.len() - 2;
    let mut i = records_start;

    while i + record_hdr_len <= end {
        let tag = raw[i];
        let index = raw[i + 1];
        let length = raw[i + 5] as usize;
        let data_start = i + record_hdr_len;
        let data_end = data_start + length;
        if data_end > end {
            break;
        }
        let data = &raw[data_start..data_end];

        if tag == 0x01 && data.len() > 4 {
            let fw_str = format!(
                "{:02}.{:02}.{:02}.{:02}",
                data[3], data[2], data[1], data[0]
            );
            let serial = parse_ascii_string(&data[4..]);
            if index == 0 {
                if status.rx.is_none() {
                    status.rx = Some(RxInfo::default());
                }
                if let Some(ref mut rx) = status.rx {
                    rx.serial = Some(serial);
                    rx.firmware = Some(fw_str);
                }
            } else if index == 1 || index == 2 {
                let idx = (index - 1) as usize;
                if status.tx[idx].is_none() {
                    status.tx[idx] = Some(TxInfo {
                        unit: index as i32,
                        connected: true,
                        ..Default::default()
                    });
                }
                if let Some(ref mut t) = status.tx[idx] {
                    t.serial = Some(serial);
                    t.firmware = Some(fw_str);
                }
            }
        } else if tag == 0x04 && !data.is_empty() {
            let bt_id = parse_ascii_string(data);
            if index == 0 {
                if status.rx.is_none() {
                    status.rx = Some(RxInfo::default());
                }
                if let Some(ref mut rx) = status.rx {
                    rx.bluetooth_id = Some(bt_id);
                }
            } else if index == 1 || index == 2 {
                let idx = (index - 1) as usize;
                if let Some(ref mut t) = status.tx[idx] {
                    t.bluetooth_id = Some(bt_id);
                }
            }
        } else if tag == 0x05 && data.len() == 1 && (index == 1 || index == 2) {
            let idx = (index - 1) as usize;
            if let Some(ref mut t) = status.tx[idx] {
                t.level = Some(data[0] as i32);
            }
        } else if tag == 0x06 && !data.is_empty() {
            let prod = parse_ascii_string(data);
            if index == 0 {
                if status.rx.is_none() {
                    status.rx = Some(RxInfo::default());
                }
                if let Some(ref mut rx) = status.rx {
                    rx.product_name = Some(prod);
                }
            } else if index == 1 || index == 2 {
                let idx = (index - 1) as usize;
                if let Some(ref mut t) = status.tx[idx] {
                    t.product_name = Some(prod);
                }
            }
        }
        i = data_end;
    }
}

pub fn decode_version_response(status: &mut DeviceStatus, raw: &[u8]) {
    if raw.len() < 35 {
        return;
    }
    if (raw[8] & 0x80) == 0 || raw[9] != 0x00 || raw[10] != 0x01 {
        return;
    }
    let payload = &raw[11..raw.len() - 2];
    if payload.len() < 24 {
        return;
    }
    let model_str = parse_ascii_string(&payload[2..22]);
    if status.rx.is_none() {
        status.rx = Some(RxInfo::default());
    }
    if let Some(ref mut rx) = status.rx {
        if !model_str.is_empty() {
            rx.hardware_model = Some(model_str);
        }
        if payload.len() >= 30 {
            let fw = &payload[22..26];
            let hw = &payload[26..30];
            if rx.firmware.is_none() {
                rx.firmware = Some(format!(
                    "{:02}.{:02}.{:02}.{:02}",
                    fw[3], fw[2], fw[1], fw[0]
                ));
            }
            rx.hardware_revision = Some(format!(
                "{:02}.{:02}.{:02}.{:02}",
                hw[0], hw[1], hw[2], hw[3]
            ));
        }
    }
}

pub fn apply_frame_to_status(frame: &DumlFrame, status: &mut DeviceStatus) {
    let raw = &frame.raw;
    if (frame.pack_type & 0x80) != 0
        && frame.cmd_set == CMD_SET_GENERAL
        && frame.cmd_id == CMD_ID_GET_VERSION
    {
        decode_version_response(status, raw);
        return;
    }
    if frame.pack_type == 0x00
        && frame.cmd_set == CMD_SET_MIC
        && frame.cmd_id == CMD_ID_HEARTBEAT_PUSH
    {
        status.aux_5b05_hex = Some(hex_encode(&frame.payload));
        return;
    }
    if frame.pack_type == 0x00 && frame.cmd_set == CMD_SET_AUX && frame.cmd_id == CMD_ID_AUX_74 {
        status.aux_0474_hex = Some(hex_encode(&frame.payload));
        return;
    }

    if let Some(dialect) = detect_dialect(raw) {
        if dialect == "v1" {
            decode_v1_heartbeat(status, raw);
        } else if dialect == "v2" {
            if !decode_v2_status(status, raw) {
                decode_v2_records(status, raw);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct SettingSpec {
    pub name: &'static str,
    pub target: &'static str, // "rx", "tx", "tx_unit"
    pub v1_cmd_id: Option<u16>,
    pub v2_cmd_id: u16,
    pub allowed_values: &'static [(&'static str, u8)],
    pub description: &'static str,
}

pub const SETTINGS_SPEC: &[SettingSpec] = &[
    SettingSpec {
        name: "noise-cancel",
        target: "tx",
        v1_cmd_id: Some(0x031D),
        v2_cmd_id: 0x0037,
        allowed_values: &[("basic", 0x00), ("strong", 0x01)],
        description: "Active Noise Cancellation strength (basic | strong)",
    },
    SettingSpec {
        name: "noise-cancel-power",
        target: "tx",
        v1_cmd_id: None,
        v2_cmd_id: 0x0038,
        allowed_values: &[("off", 0x00), ("on", 0x01)],
        description: "Noise Cancelling power (on | off, v2 firmware)",
    },
    SettingSpec {
        name: "noise-cancel-button",
        target: "tx",
        v1_cmd_id: None,
        v2_cmd_id: 0x000F,
        allowed_values: &[("off", 0x00), ("on", 0x01)],
        description: "Toggle NC via TX Power Button (on | off, v2 firmware)",
    },
    SettingSpec {
        name: "low-cut",
        target: "tx",
        v1_cmd_id: Some(0x0303),
        v2_cmd_id: 0x0003,
        allowed_values: &[("off", 0x00), ("on", 0x01)],
        description: "Low-cut (high-pass wind/rumble filter) on transmitters (on | off)",
    },
    SettingSpec {
        name: "stereo",
        target: "rx",
        v1_cmd_id: Some(0x0008),
        v2_cmd_id: 0x0008,
        allowed_values: &[
            ("mono", 0x00),
            ("off", 0x00),
            ("stereo", 0x02),
            ("on", 0x02),
        ],
        description: "Audio channel routing mode (mono | stereo)",
    },
    SettingSpec {
        name: "safety-track",
        target: "rx",
        v1_cmd_id: Some(0x0021),
        v2_cmd_id: 0x0021,
        allowed_values: &[("off", 0x00), ("on", 0x01)],
        description: "Dual mono -6 dB backup safety track on right channel (on | off)",
    },
    SettingSpec {
        name: "clip-limiter",
        target: "rx",
        v1_cmd_id: Some(0x001E),
        v2_cmd_id: 0x001E,
        allowed_values: &[("off", 0x00), ("on", 0x01)],
        description: "Automatic audio clipping limiter (on | off)",
    },
    SettingSpec {
        name: "auto-off-15m",
        target: "rx",
        v1_cmd_id: Some(0x0010),
        v2_cmd_id: 0x0010,
        allowed_values: &[("off", 0x00), ("on", 0x01)],
        description: "Receiver 15-minute inactivity auto power-off (on | off)",
    },
    SettingSpec {
        name: "tx-auto-off-15m",
        target: "tx",
        v1_cmd_id: None,
        v2_cmd_id: 0x0010,
        allowed_values: &[("off", 0x00), ("on", 0x01)],
        description: "Transmitter 15-minute inactivity auto power-off (on | off, v2 firmware)",
    },
    SettingSpec {
        name: "camera-power",
        target: "rx",
        v1_cmd_id: Some(0x0020),
        v2_cmd_id: 0x0020,
        allowed_values: &[("off", 0x00), ("on", 0x01)],
        description: "Auto power on/off synchronized with camera/host USB power (on | off)",
    },
    SettingSpec {
        name: "plug-free",
        target: "rx",
        v1_cmd_id: Some(0x0023),
        v2_cmd_id: 0x0023,
        allowed_values: &[("off", 0x00), ("on", 0x01)],
        description: "Allow external speaker playback while receiver stays plugged in (on | off)",
    },
    SettingSpec {
        name: "mic-leds",
        target: "tx",
        v1_cmd_id: Some(0x030A),
        v2_cmd_id: 0x000A,
        allowed_values: &[("on", 0x00), ("off", 0x02)],
        description: "Transmitter status indicator LEDs (on | off)",
    },
    SettingSpec {
        name: "voice-tone",
        target: "tx_unit",
        v1_cmd_id: None,
        v2_cmd_id: 0x0029,
        allowed_values: &[("standard", 0x00), ("rich", 0x01), ("bright", 0x02)],
        description: "Voice tone preset (standard | rich | bright, v2 firmware)",
    },
];

pub fn find_setting_spec(name: &str) -> Option<&'static SettingSpec> {
    SETTINGS_SPEC.iter().find(|s| s.name == name)
}

pub fn build_frame(
    cmd_set: u8,
    cmd_id: u8,
    payload: &[u8],
    seq: u16,
    dialect: &str,
    sender: u8,
    receiver: u8,
    pack_type: u8,
) -> Vec<u8> {
    let total_len = 13 + payload.len();
    let mut hdr4 = vec![
        SOF,
        (total_len & 0xFF) as u8,
        (((total_len >> 8) & 0x03) as u8) | VERSION,
    ];
    let hdr_crc = crc8(&hdr4, CRC8_INIT);
    hdr4.push(hdr_crc);

    let seq_bytes = seq.to_le_bytes();
    let mut body = hdr4;
    body.extend_from_slice(&[
        sender,
        receiver,
        seq_bytes[0],
        seq_bytes[1],
        pack_type,
        cmd_set,
        cmd_id,
    ]);
    body.extend_from_slice(payload);

    let f_crc = calc_frame_crc16_bytes(&body, dialect);
    body.extend_from_slice(&f_crc);
    body
}

pub fn build_get_version_packet(seq: u16, _dialect: &str) -> Vec<u8> {
    let length = 13usize;
    let mut hdr = vec![
        SOF,
        (length & 0xFF) as u8,
        (((length >> 8) & 0x03) as u8) | VERSION,
    ];
    let h_crc = crc8(&hdr, CRC8_INIT);
    hdr.push(h_crc);
    hdr.extend_from_slice(&[
        0x02,
        0x5A,
        (seq & 0xFF) as u8,
        ((seq >> 8) & 0xFF) as u8,
        0x40,
        0x00,
        0x01,
    ]);
    let crc = crc16(&hdr, CRC16_V2_INIT);
    hdr.extend_from_slice(&crc.to_le_bytes());
    hdr
}

pub fn build_command_packet(
    setting: &str,
    value: &str,
    seq: u16,
    dialect: &str,
    tx_unit: Option<i32>,
) -> Result<Vec<u8>, String> {
    let spec = find_setting_spec(setting)
        .ok_or_else(|| format!("Unknown setting '{}'", setting))?;
    let val_norm = value.trim().to_lowercase();
    let &(_, wire_val) = spec
        .allowed_values
        .iter()
        .find(|(k, _)| *k == val_norm)
        .ok_or_else(|| format!("Invalid value '{}' for '{}'", value, setting))?;

    if dialect == "v1" {
        let cmd_id = spec.v1_cmd_id.ok_or_else(|| {
            format!(
                "Setting '{}' is not modifiable over USB on v1 firmware",
                setting
            )
        })?;
        let mut body = vec![
            SOF,
            19,
            0x04,
            0x03,
            0x02,
            0x5A,
            (seq & 0xFF) as u8,
            ((seq >> 8) & 0xFF) as u8,
            0x40,
            0x5B,
            0x01,
            0x00,
            ((cmd_id >> 8) & 0xFF) as u8,
            (cmd_id & 0xFF) as u8,
            0x00,
            0x01,
            wire_val,
        ];
        let crc = calc_frame_crc16_bytes(&body, "v1");
        body.extend_from_slice(&crc);
        Ok(body)
    } else {
        let cmd_id = spec.v2_cmd_id;
        let target: u16 = match spec.target {
            "rx" => 0x0000,
            "tx" => 0xFFFF,
            "tx_unit" => {
                let u = tx_unit.unwrap_or(1);
                if u != 1 && u != 2 {
                    return Err("Target TX unit must be 1 or 2 for this setting".to_string());
                }
                u as u16
            }
            _ => 0xFFFF,
        };
        let mut body = vec![
            SOF,
            22,
            0x04,
            0xFC,
            0x02,
            0x5A,
            (seq & 0xFF) as u8,
            ((seq >> 8) & 0xFF) as u8,
            0x40,
            0x5B,
            0x01,
            0x02,
            (target & 0xFF) as u8,
            ((target >> 8) & 0xFF) as u8,
            0x00,
            0x00,
            (cmd_id & 0xFF) as u8,
            ((cmd_id >> 8) & 0xFF) as u8,
            0x01,
            wire_val,
        ];
        let crc = calc_frame_crc16_bytes(&body, "v2");
        body.extend_from_slice(&crc);
        Ok(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc_and_frame_roundtrip() {
        let pkt = build_get_version_packet(42, "v1");
        let decoded = decode_frame(&pkt).expect("should decode GetVersion packet");
        assert_eq!(decoded.seq, 42);
        assert_eq!(decoded.cmd_set, CMD_SET_GENERAL);
        assert_eq!(decoded.cmd_id, CMD_ID_GET_VERSION);

        let cmd1 = build_command_packet("noise-cancel", "strong", 99, "v1", None)
            .expect("should build v1 command");
        let dec1 = decode_frame(&cmd1).expect("should decode v1 command");
        assert_eq!(dec1.dialect, "v1");
        assert_eq!(dec1.cmd_set, CMD_SET_MIC);
        assert_eq!(dec1.cmd_id, 0x01);

        let cmd2 = build_command_packet("noise-cancel", "strong", 99, "v2", Some(1))
            .expect("should build v2 command");
        let dec2 = decode_frame(&cmd2).expect("should decode v2 command");
        assert_eq!(dec2.dialect, "v2");
        assert_eq!(dec2.cmd_set, CMD_SET_MIC);
        assert_eq!(dec2.cmd_id, 0x01);
    }

    #[test]
    fn test_v1_live_heartbeat_decode() {
        let hex_str = "5546048b5a02ed9b005b03003900b4263a00010100380e385044584e35453033354a5058350040000000000000003220010100380e385043584e354830323432395448019e3c";
        let mut raw = Vec::new();
        for chunk in hex_str.as_bytes().chunks(2) {
            raw.push(u8::from_str_radix(&String::from_utf8_lossy(chunk), 16).unwrap());
        }
        let frame = decode_frame(&raw).expect("should decode live v1 heartbeat");
        assert_eq!(frame.dialect, "v1");
        let mut st = DeviceStatus::default();
        apply_frame_to_status(&frame, &mut st);
        assert!(st.connected);
        assert_eq!(st.dialect, "v1");
        let rx = st.rx.expect("should have rx");
        assert_eq!(rx.serial.as_deref(), Some("8PCXN5H02429TH"));
        assert_eq!(rx.firmware.as_deref(), Some("01.01.00.56"));
        let tx1 = st.tx[0].as_ref().expect("should have tx1");
        assert!(tx1.connected);
        assert_eq!(tx1.serial.as_deref(), Some("8PDXN5E035JPX5"));
        assert_eq!(tx1.battery_percent, Some(100));
    }

    #[test]
    fn test_battery_and_gain_helpers() {
        assert_eq!(battery_code_to_percent(Some(1)), Some(100));
        assert_eq!(battery_code_to_percent(Some(7)), Some(5));
        assert_eq!(gain_step_to_db(3), Some(0));
        assert_eq!(gain_step_to_db(5), Some(12));
    }
}

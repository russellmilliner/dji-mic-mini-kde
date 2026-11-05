use serde::{Deserialize, Serialize};
use std::fs;
use std::process::Command;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AlsaCardInfo {
    pub card_index: Option<i32>,
    pub card_id: Option<String>,
    pub card_name: Option<String>,
    pub usb_bus_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlsaVolumeInfo {
    pub control: String,
    pub percent: i32,
    pub db: Option<String>,
    pub muted: bool,
}

impl Default for AlsaVolumeInfo {
    fn default() -> Self {
        Self {
            control: "Mic".to_string(),
            percent: 100,
            db: Some("0.00dB".to_string()),
            muted: false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipewireStreamInfo {
    pub wpctl_id: Option<i32>,
    pub pactl_id: Option<i32>,
    pub node_name: Option<String>,
    pub description: Option<String>,
    pub sample_spec: Option<String>,
    pub channel_map: Option<String>,
    pub stream_state: Option<String>,
}

fn run_cmd(args: &[&str]) -> Option<String> {
    let out = Command::new(args[0]).args(&args[1..]).output().ok()?;
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}

pub fn get_alsa_card_info() -> AlsaCardInfo {
    let mut info = AlsaCardInfo::default();
    let Ok(txt) = fs::read_to_string("/proc/asound/cards") else {
        return info;
    };
    let lines: Vec<&str> = txt.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.to_uppercase().contains("DJI MIC MINI") || line.contains("[MINI") {
            let trimmed = line.trim();
            let idx_str: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(idx) = idx_str.parse::<i32>() {
                info.card_index = Some(idx);
            }
            if let (Some(lb), Some(rb)) = (trimmed.find('['), trimmed.find(']')) {
                if rb > lb {
                    info.card_id = Some(trimmed[lb + 1..rb].trim().to_string());
                }
            }
            if let Some(dash) = trimmed.find(" - ") {
                info.card_name = Some(trimmed[dash + 3..].trim().to_string());
            }
            if i + 1 < lines.len() {
                let next_l = lines[i + 1].trim();
                if let Some(at_pos) = next_l.find(" at ") {
                    let after = &next_l[at_pos + 4..];
                    let bus_tok = after.split(',').next().unwrap_or("").trim();
                    if !bus_tok.is_empty() {
                        info.usb_bus_path = Some(bus_tok.to_string());
                    }
                }
            }
            break;
        }
    }
    info
}

pub fn find_alsa_card() -> Option<String> {
    let info = get_alsa_card_info();
    info.card_index.map(|i| i.to_string())
}

pub fn get_alsa_volume() -> Option<AlsaVolumeInfo> {
    let card = find_alsa_card()?;
    for ctrl in ["Mic", "Capture", "PCM"] {
        let Some(out) = run_cmd(&["amixer", "-c", &card, "sget", ctrl]) else {
            continue;
        };
        if !out.contains("Limits:") && !out.contains("Mono:") && !out.contains("Front Left:") {
            continue;
        }

        let mut pct: Option<i32> = None;
        let mut db: Option<String> = None;
        let mut muted = false;

        for token in out.split('[') {
            if let Some(end) = token.find(']') {
                let inside = &token[..end];
                if let Some(num_s) = inside.strip_suffix('%') {
                    if pct.is_none() {
                        if let Ok(v) = num_s.trim().parse::<i32>() {
                            pct = Some(v);
                        }
                    }
                } else if inside.ends_with("dB") {
                    if db.is_none() {
                        db = Some(inside.trim().to_string());
                    }
                } else if inside == "off" {
                    muted = true;
                }
            }
        }

        if let Some(p) = pct {
            return Some(AlsaVolumeInfo {
                control: ctrl.to_string(),
                percent: p,
                db,
                muted,
            });
        }
    }
    None
}

pub fn set_alsa_mute(mute: bool) -> bool {
    let Some(card) = find_alsa_card() else {
        return false;
    };
    let state_arg = if mute { "nocap" } else { "cap" };
    for ctrl in ["Mic", "Capture"] {
        if let Some(out) = run_cmd(&["amixer", "-c", &card, "sset", ctrl, state_arg]) {
            if out.contains("[on]") || out.contains("[off]") {
                return true;
            }
        }
    }
    false
}

pub fn toggle_alsa_mute() -> Option<bool> {
    let cur = get_alsa_volume()?;
    let target = !cur.muted;
    if set_alsa_mute(target) {
        Some(target)
    } else {
        None
    }
}

pub fn set_alsa_volume(volume_spec: &str) -> bool {
    let Some(card) = find_alsa_card() else {
        return false;
    };
    for ctrl in ["Mic", "Capture"] {
        if let Some(out) = run_cmd(&["amixer", "-c", &card, "sset", ctrl, volume_spec]) {
            if out.contains('%') {
                return true;
            }
        }
    }
    false
}

pub fn get_pipewire_source_id() -> Option<i32> {
    let out = run_cmd(&["wpctl", "status"])?;
    let mut in_sources = false;
    for line in out.lines() {
        if line.contains("Sources:") {
            in_sources = true;
            continue;
        }
        if in_sources && (line.contains("Filters:") || line.contains("Streams:") || line.contains("Video")) {
            in_sources = false;
        }
        if in_sources && line.to_uppercase().contains("DJI MIC MINI") {
            for tok in line.split_whitespace() {
                if let Some(num_s) = tok.strip_suffix('.') {
                    if let Ok(id) = num_s.parse::<i32>() {
                        return Some(id);
                    }
                }
            }
        }
    }
    None
}

pub fn get_pipewire_stream_details() -> PipewireStreamInfo {
    let mut info = PipewireStreamInfo {
        wpctl_id: get_pipewire_source_id(),
        ..Default::default()
    };

    let Some(out) = run_cmd(&["pactl", "list", "sources"]) else {
        return info;
    };
    for block in out.split("Source #") {
        if !block.to_uppercase().contains("DJI_MIC_MINI")
            && !block.to_uppercase().contains("DJI MIC MINI")
        {
            continue;
        }
        let mut lines = block.lines();
        if let Some(first) = lines.next() {
            if let Ok(pid) = first.trim().parse::<i32>() {
                info.pactl_id = Some(pid);
            }
        }
        for line in lines {
            let s = line.trim();
            if let Some(v) = s.strip_prefix("State:") {
                info.stream_state = Some(v.trim().to_string());
            } else if let Some(v) = s.strip_prefix("Name:") {
                info.node_name = Some(v.trim().to_string());
            } else if let Some(v) = s.strip_prefix("Description:") {
                info.description = Some(v.trim().to_string());
            } else if let Some(v) = s.strip_prefix("Sample Specification:") {
                info.sample_spec = Some(v.trim().to_string());
            } else if let Some(v) = s.strip_prefix("Channel Map:") {
                info.channel_map = Some(v.trim().to_string());
            }
        }
        break;
    }

    info
}

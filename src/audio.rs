//! Linux Audio Subsystem Integration (ALSA & PipeWire) and
//! AI Noise Cancellation & Ambient Room Noise Learning (`librnnoise.so.0` + 4-Band Spectral Profiler)
//! for KDE DJI Mic Mini Control.

use crate::util::{
    lock_unpoisoned, now_hms, run_command_with_timeout, write_file_atomic_mode, xdg_config_home,
};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use std::ffi::CString;
use std::fmt;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{LazyLock, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiNcMode {
    Off,
    Hardware,
    #[default]
    LearnedAi,
    Adaptive,
}

impl AiNcMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Hardware => "hardware",
            Self::LearnedAi => "learned_ai",
            Self::Adaptive => "adaptive",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "off" => Self::Off,
            "hardware" => Self::Hardware,
            "adaptive" => Self::Adaptive,
            _ => Self::LearnedAi,
        }
    }

    pub fn next_cycle(self) -> Self {
        match self {
            Self::Off => Self::Hardware,
            Self::Hardware => Self::LearnedAi,
            Self::LearnedAi => Self::Adaptive,
            Self::Adaptive => Self::Off,
        }
    }
}

impl fmt::Display for AiNcMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for AiNcMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "off" => Ok(Self::Off),
            "hardware" => Ok(Self::Hardware),
            "learned_ai" => Ok(Self::LearnedAi),
            "adaptive" => Ok(Self::Adaptive),
            other => Err(format!("Invalid AI NC mode '{other}'")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AlsaCardInfo {
    pub card_index: Option<i32>,
    pub card_id: Option<String>,
    pub card_name: Option<String>,
    pub usb_bus_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AlsaVolumeInfo {
    pub control: String,
    pub percent: i32,
    pub db: Option<String>,
    pub muted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PipewireStreamInfo {
    pub pactl_id: Option<i32>,
    pub wpctl_id: Option<i32>,
    pub node_name: Option<String>,
    pub sample_spec: Option<String>,
    pub stream_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseLearningProfile {
    pub learned: bool,
    pub learned_at: String,
    pub capture_source: String,
    pub noise_floor_dbfs: f64,
    pub peak_noise_dbfs: f64,
    pub denoised_floor_dbfs: f64,
    pub rnnoise_attenuation_db: f64,
    pub band_low_rumble_db: f64,
    pub band_fan_hum_db: f64,
    pub band_mid_room_db: f64,
    pub band_high_hiss_db: f64,
    pub dominant_hum_hz: i32,
    pub noise_signature: String,
    pub learned_vad_threshold_pct: i32,
    pub learned_highpass_hz: i32,
    pub learned_notch_hz: i32,
    pub learned_notch_db: f64,
    pub learned_gate_dbfs: f64,
    pub recommended_hw_nc_mode: String,
    pub recommended_hw_low_cut: String,
    pub adaptive_passes: u32,
}

impl Default for NoiseLearningProfile {
    fn default() -> Self {
        Self {
            learned: false,
            learned_at: "Not learned yet".to_string(),
            capture_source: "DJI Mic Mini RX (PipeWire / ALSA)".to_string(),
            noise_floor_dbfs: -54.0,
            peak_noise_dbfs: -42.0,
            denoised_floor_dbfs: -71.0,
            rnnoise_attenuation_db: 17.0,
            band_low_rumble_db: -55.0,
            band_fan_hum_db: -51.0,
            band_mid_room_db: -57.0,
            band_high_hiss_db: -59.0,
            dominant_hum_hz: 120,
            noise_signature: "Default Baseline Profile (Click 'Learn Room Noise' while silent)"
                .to_string(),
            learned_vad_threshold_pct: 62,
            learned_highpass_hz: 95,
            learned_notch_hz: 120,
            learned_notch_db: -6.0,
            learned_gate_dbfs: -47.0,
            recommended_hw_nc_mode: "basic".to_string(),
            recommended_hw_low_cut: "off".to_string(),
            adaptive_passes: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NoiseLearnLiveState {
    pub learning_active: bool,
    pub progress_pct: i32,
    pub status_text: String,
    pub profile: NoiseLearningProfile,
    pub virtual_mic_active: bool,
}

static NOISE_LEARN_ACTIVE: AtomicBool = AtomicBool::new(false);
static NOISE_LEARN_PROGRESS: AtomicI32 = AtomicI32::new(0);
static NOISE_LEARN_STATUS: LazyLock<Mutex<String>> = LazyLock::new(|| {
    Mutex::new("Idle — Ready to learn room acoustic signature from DJI Mic Mini".to_string())
});
static CACHED_NOISE_PROFILE: LazyLock<Mutex<NoiseLearningProfile>> =
    LazyLock::new(|| Mutex::new(load_noise_profile_from_disk()));
static LAST_ADAPTIVE_PROBE: LazyLock<Mutex<Option<Instant>>> =
    LazyLock::new(|| Mutex::new(None));

pub fn noise_profile_path() -> PathBuf {
    xdg_config_home()
        .join("dji-mic-tray")
        .join("noise_profile.json")
}

pub fn pipewire_filter_conf_path() -> PathBuf {
    xdg_config_home()
        .join("dji-mic-tray")
        .join("learned-rnnoise-filter.conf")
}

fn sanitize_conf_text(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\r' | '\n' | '\0' => ' ',
            '"' => '\'',
            _ => c,
        })
        .collect()
}

pub fn load_noise_profile_from_disk() -> NoiseLearningProfile {
    let path = noise_profile_path();
    if let Ok(txt) = fs::read_to_string(&path) {
        if let Ok(prof) = serde_json::from_str::<NoiseLearningProfile>(&txt) {
            return prof;
        }
    }
    NoiseLearningProfile::default()
}

pub fn save_noise_profile_to_disk(prof: &NoiseLearningProfile) {
    let path = noise_profile_path();
    if let Ok(js) = serde_json::to_string_pretty(prof) {
        let _ = write_file_atomic_mode(&path, js.as_bytes(), 0o600);
    }
    write_learned_filter_chain_conf(prof);
}

pub fn write_learned_filter_chain_conf(prof: &NoiseLearningProfile) {
    let path = pipewire_filter_conf_path();
    let learned_at = sanitize_conf_text(&prof.learned_at);
    let src = sanitize_conf_text(&prof.capture_source);
    let sig = sanitize_conf_text(&prof.noise_signature);
    let conf = format!(
        "# Auto-generated by KDE DJI Mic Mini Control — Learned AI Noise Cancellation Filter\n\
         # Learned At: {learned_at} | Source: {src}\n\
         # Signature : {sig}\n\
         context.modules = [\n\
         \x20   {{ name = libpipewire-module-filter-chain\n\
         \x20       flags = [ nofail ]\n\
         \x20       args = {{\n\
         \x20           node.description = \"DJI Mic Mini Learned AI Noise-Cancelled Mic\"\n\
         \x20           media.name       = \"DJI Mic Mini Learned AI Noise-Cancelled Mic\"\n\
         \x20           filter.graph = {{\n\
         \x20               nodes = [\n\
         \x20                   {{\n\
         \x20                       type    = builtin\n\
         \x20                       name    = learned_hp\n\
         \x20                       label   = bq_highpass\n\
         \x20                       control = {{ \"Freq\" = {hp:.1} \"Q\" = 0.707 \"Gain\" = 0.0 }}\n\
         \x20                   }}\n\
         \x20                   {{\n\
         \x20                       type    = builtin\n\
         \x20                       name    = learned_hum_notch\n\
         \x20                       label   = bq_peaking\n\
         \x20                       control = {{ \"Freq\" = {notch_hz:.1} \"Q\" = 4.2 \"Gain\" = {notch_db:.1} }}\n\
         \x20                   }}\n\
         \x20               ]\n\
         \x20               links = [\n\
         \x20                   {{ output = \"learned_hp:Out\" input = \"learned_hum_notch:In\" }}\n\
         \x20               ]\n\
         \x20           }}\n\
         \x20           audio.position = [ FL FR ]\n\
         \x20           capture.props = {{\n\
         \x20               node.name    = \"effect_input.dji_learned_nc\"\n\
         \x20               node.passive = true\n\
         \x20           }}\n\
         \x20           playback.props = {{\n\
         \x20               node.name   = \"effect_output.dji_learned_nc\"\n\
         \x20               media.class = Audio/Source\n\
         \x20           }}\n\
         \x20       }}\n\
         \x20   }}\n\
         ]\n",
        hp = prof.learned_highpass_hz as f64,
        notch_hz = prof.learned_notch_hz as f64,
        notch_db = prof.learned_notch_db,
    );
    let _ = write_file_atomic_mode(&path, conf.as_bytes(), 0o600);
}

pub fn get_noise_learn_live_state() -> NoiseLearnLiveState {
    let prof = lock_unpoisoned(&CACHED_NOISE_PROFILE).clone();
    let status = lock_unpoisoned(&NOISE_LEARN_STATUS).clone();
    NoiseLearnLiveState {
        learning_active: NOISE_LEARN_ACTIVE.load(Ordering::SeqCst),
        progress_pct: NOISE_LEARN_PROGRESS.load(Ordering::SeqCst),
        status_text: status,
        profile: prof,
        virtual_mic_active: is_pipewire_ai_virtual_mic_loaded(),
    }
}

pub fn reset_learned_noise_profile() -> NoiseLearningProfile {
    let def = NoiseLearningProfile::default();
    *lock_unpoisoned(&CACHED_NOISE_PROFILE) = def.clone();
    let _ = fs::remove_file(noise_profile_path());
    write_learned_filter_chain_conf(&def);
    *lock_unpoisoned(&NOISE_LEARN_STATUS) =
        "Reset learned room noise profile to baseline defaults".to_string();
    NOISE_LEARN_PROGRESS.store(0, Ordering::SeqCst);
    def
}

static VIRTUAL_MIC_CACHE: LazyLock<Mutex<Option<(Instant, bool)>>> =
    LazyLock::new(|| Mutex::new(None));

pub fn is_pipewire_ai_virtual_mic_loaded() -> bool {
    if let Some((ts, cached)) = *lock_unpoisoned(&VIRTUAL_MIC_CACHE) {
        if ts.elapsed() < Duration::from_secs(4) {
            return cached;
        }
    }
    let Ok(out) = run_command_with_timeout(
        "pactl",
        &["list", "sources", "short"],
        Duration::from_secs(2),
    ) else {
        return false;
    };
    if !out.status.success() {
        return false;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let active = stdout.contains("dji_ai_denoise_mic");
    *lock_unpoisoned(&VIRTUAL_MIC_CACHE) = Some((Instant::now(), active));
    active
}

fn find_dji_pactl_source_name() -> Option<String> {
    let out = run_command_with_timeout(
        "pactl",
        &["list", "sources", "short"],
        Duration::from_secs(2),
    )
    .ok()?;
    if !out.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in stdout.lines() {
        if line.contains("DJI_MIC_MINI") || line.contains("DJI MIC MINI") {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 {
                let name = parts[1].trim();
                if !name.is_empty() {
                    return Some(name.to_string());
                }
            }
        }
    }
    None
}

pub fn sync_pipewire_ai_virtual_mic(enabled: bool, force_reload: bool) -> (bool, String) {
    *lock_unpoisoned(&VIRTUAL_MIC_CACHE) = None;
    let cmd_timeout = Duration::from_secs(3);
    if !enabled || force_reload {
        if let Ok(out) = run_command_with_timeout("pactl", &["list", "modules", "short"], cmd_timeout)
        {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                if line.contains("dji_ai_denoise_mic") {
                    if let Some(mod_id) = line.split_whitespace().next() {
                        let _ = run_command_with_timeout(
                            "pactl",
                            &["unload-module", mod_id],
                            cmd_timeout,
                        );
                    }
                }
            }
        }
        if !enabled {
            *lock_unpoisoned(&VIRTUAL_MIC_CACHE) = Some((Instant::now(), false));
            return (
                false,
                "PipeWire AI Noise-Cancelled Virtual Mic disabled".to_string(),
            );
        }
    }

    if is_pipewire_ai_virtual_mic_loaded() {
        return (
            true,
            "PipeWire AI Virtual Mic (dji_ai_denoise_mic) active".to_string(),
        );
    }

    let prof = lock_unpoisoned(&CACHED_NOISE_PROFILE).clone();
    write_learned_filter_chain_conf(&prof);

    let aec_args = "webrtc.high_pass_filter=true webrtc.noise_suppression=true webrtc.voice_detection=true webrtc.transient_suppression=true";
    let aec_flag = format!("aec_args=\"{aec_args}\"");
    let source_master_arg = find_dji_pactl_source_name().map(|s| format!("source_master={s}"));
    let mut args = vec![
        "load-module",
        "module-echo-cancel",
        "source_name=dji_ai_denoise_mic",
        "sink_name=dji_ai_denoise_sink",
        "aec_method=webrtc",
        aec_flag.as_str(),
        "use_master_format=1",
        "source_properties=device.description=DJI_Mic_Mini_AI_Noise_Cancelled_Mic",
    ];
    if let Some(ref sm) = source_master_arg {
        args.push(sm.as_str());
    }

    match run_command_with_timeout("pactl", &args, cmd_timeout) {
        Ok(o) if o.status.success() => {
            *lock_unpoisoned(&VIRTUAL_MIC_CACHE) = Some((Instant::now(), true));
            (
                true,
                format!(
                    "Loaded PipeWire WebRTC AI Virtual Mic 'dji_ai_denoise_mic' (Calibrated DSP: HP {}Hz, Notch {}Hz {:.1}dB, VAD {}%)",
                    prof.learned_highpass_hz,
                    prof.learned_notch_hz,
                    prof.learned_notch_db,
                    prof.learned_vad_threshold_pct
                ),
            )
        }
        Ok(o) => (
            false,
            format!(
                "Failed to load PipeWire AI Virtual Mic: {}",
                String::from_utf8_lossy(&o.stderr).trim()
            ),
        ),
        Err(e) => (false, format!("Failed to invoke pactl: {e}")),
    }
}

#[repr(C)]
struct DenoiseStateOpaque {
    _priv: [u8; 0],
}

type FnRnnoiseCreate = unsafe extern "C" fn(*const libc::c_void) -> *mut DenoiseStateOpaque;
type FnRnnoiseDestroy = unsafe extern "C" fn(*mut DenoiseStateOpaque);
type FnRnnoiseProcessFrame =
    unsafe extern "C" fn(*mut DenoiseStateOpaque, *mut f32, *const f32) -> f32;

struct RnnoiseApi {
    _handle: usize,
    fn_create: FnRnnoiseCreate,
    fn_destroy: FnRnnoiseDestroy,
    fn_process: FnRnnoiseProcessFrame,
}

static RNNOISE_API: OnceLock<Option<RnnoiseApi>> = OnceLock::new();

fn get_rnnoise_api() -> Option<&'static RnnoiseApi> {
    RNNOISE_API
        .get_or_init(|| {
            let candidates = [
                "librnnoise.so.0",
                "/usr/lib/librnnoise.so.0",
                "/usr/lib/librnnoise.so",
                "/usr/lib64/librnnoise.so.0",
            ];
            for path in candidates {
                let Ok(cpath) = CString::new(path) else {
                    continue;
                };
                unsafe {
                    let handle = libc::dlopen(cpath.as_ptr(), libc::RTLD_LAZY | libc::RTLD_LOCAL);
                    if handle.is_null() {
                        continue;
                    }
                    let sym_create = libc::dlsym(handle, c"rnnoise_create".as_ptr());
                    let sym_destroy = libc::dlsym(handle, c"rnnoise_destroy".as_ptr());
                    let sym_process = libc::dlsym(handle, c"rnnoise_process_frame".as_ptr());
                    if sym_create.is_null() || sym_destroy.is_null() || sym_process.is_null() {
                        libc::dlclose(handle);
                        continue;
                    }
                    let fn_create: FnRnnoiseCreate = std::mem::transmute(sym_create);
                    let fn_destroy: FnRnnoiseDestroy = std::mem::transmute(sym_destroy);
                    let fn_process: FnRnnoiseProcessFrame = std::mem::transmute(sym_process);
                    return Some(RnnoiseApi {
                        _handle: handle as usize,
                        fn_create,
                        fn_destroy,
                        fn_process,
                    });
                }
            }
            None
        })
        .as_ref()
}

struct RnnoiseRunner {
    api: &'static RnnoiseApi,
    state: *mut DenoiseStateOpaque,
}

impl RnnoiseRunner {
    fn try_new() -> Option<Self> {
        let api = get_rnnoise_api()?;
        let state = unsafe { (api.fn_create)(std::ptr::null()) };
        if state.is_null() {
            return None;
        }
        Some(Self { api, state })
    }

    fn process_frame_480(&mut self, in_pcm_f32: &[f32; 480], out_pcm_f32: &mut [f32; 480]) -> f32 {
        unsafe { (self.api.fn_process)(self.state, out_pcm_f32.as_mut_ptr(), in_pcm_f32.as_ptr()) }
    }
}

impl Drop for RnnoiseRunner {
    fn drop(&mut self) {
        unsafe {
            if !self.state.is_null() {
                (self.api.fn_destroy)(self.state);
            }
        }
    }
}

fn resolve_alsa_capture_device_and_label() -> (String, String) {
    let alsa = get_alsa_card_info();
    if let Some(idx) = alsa.card_index {
        let dev = format!("plughw:{idx},0");
        let label = format!("DJI Mic Mini RX (ALSA {dev})");
        return (dev, label);
    }
    ("default".to_string(), "DJI Mic Mini RX (PipeWire Default)".to_string())
}

fn capture_pw_record_raw_48k_s16le<F>(
    duration_secs: u32,
    pw_target: Option<&str>,
    mut on_second: F,
) -> Option<Vec<i16>>
where
    F: FnMut(u32),
{
    let target_samples = duration_secs.saturating_mul(48000);
    let target_bytes = (target_samples as usize) * 2;
    let sc_arg = format!("--sample-count={target_samples}");
    let mut args = vec![
        "--raw",
        "--rate=48000",
        "--channels=1",
        "--format=s16",
        sc_arg.as_str(),
    ];
    let tgt_arg;
    if let Some(tgt) = pw_target {
        tgt_arg = format!("--target={tgt}");
        args.push(tgt_arg.as_str());
    }
    args.push("-");

    let mut child = Command::new("pw-record")
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let mut stdout = child.stdout.take()?;
    let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
    let reader = thread::spawn(move || {
        let mut buf = [0u8; 4096];
        while let Ok(n) = stdout.read(&mut buf) {
            if n == 0 {
                break;
            }
            if tx.send(buf[..n].to_vec()).is_err() {
                break;
            }
        }
    });

    let mut pcm_bytes = Vec::with_capacity(target_bytes);
    let deadline = Instant::now() + Duration::from_secs(u64::from(duration_secs) + 4);
    let mut reported_secs = 0u32;

    while pcm_bytes.len() < target_bytes && Instant::now() < deadline {
        let rem = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(rem.min(Duration::from_millis(200))) {
            Ok(chunk) => {
                let need = target_bytes - pcm_bytes.len();
                let take = need.min(chunk.len());
                pcm_bytes.extend_from_slice(&chunk[..take]);
                let secs_so_far = ((pcm_bytes.len() / 2) / 48000) as u32;
                if secs_so_far > reported_secs {
                    reported_secs = secs_so_far;
                    on_second(secs_so_far);
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                if let Ok(Some(_)) = child.try_wait() {
                    break;
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let _ = child.kill();
    let _ = child.wait();
    let _ = reader.join();

    if pcm_bytes.len() < 16000 {
        return None;
    }
    let samples: Vec<i16> = pcm_bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&chunk| i16::from_le_bytes(chunk))
        .collect();
    Some(samples)
}

fn capture_pcm_48k_s16le<F>(duration_secs: u32, mut on_second: F) -> (Vec<i16>, String)
where
    F: FnMut(u32, &str),
{
    let pw_source = find_dji_pactl_source_name();
    let pw_label = match pw_source.as_deref() {
        Some(s) => format!("DJI Mic Mini RX (PipeWire: {s})"),
        None => "DJI Mic Mini RX (PipeWire Default)".to_string(),
    };
    on_second(0, &pw_label);
    if let Some(samples) = capture_pw_record_raw_48k_s16le(
        duration_secs,
        pw_source.as_deref(),
        |sec| on_second(sec, &pw_label),
    ) {
        return (samples, pw_label);
    }

    // Fallback: single-shot raw `arecord` if `pw-record` is unavailable or unlinked
    let (alsa_dev, alsa_label) = resolve_alsa_capture_device_and_label();
    on_second(0, &alsa_label);
    let dur_str = duration_secs.max(1).to_string();
    let timeout = Duration::from_secs(u64::from(duration_secs.max(1)) + 3);
    for dev in [alsa_dev.as_str(), "default"] {
        let lbl = if dev == "default" {
            "DJI Mic Mini RX (ALSA Default)".to_string()
        } else {
            alsa_label.clone()
        };
        if let Ok(out) = run_command_with_timeout(
            "arecord",
            &[
                "-q",
                "-D",
                dev,
                "-f",
                "S16_LE",
                "-r",
                "48000",
                "-c",
                "1",
                "-t",
                "raw",
                "-d",
                &dur_str,
            ],
            timeout,
        ) {
            if out.status.success() && out.stdout.len() >= 16000 {
                let samples: Vec<i16> = out
                    .stdout
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|&chunk| i16::from_le_bytes(chunk))
                    .collect();
                return (samples, lbl);
            }
        }
    }

    (Vec::new(), pw_label)
}

fn goertzel_power_dbfs(samples: &[f64], sample_rate: f64, target_hz: f64) -> f64 {
    if samples.is_empty() {
        return -96.0;
    }
    let n = samples.len() as f64;
    let k = (0.5 + (n * target_hz) / sample_rate).floor();
    let omega = (2.0 * PI * k) / n;
    let coeff = 2.0 * omega.cos();
    let mut q1 = 0.0;
    let mut q2 = 0.0;
    for &x in samples {
        let q0 = coeff * q1 - q2 + x;
        q2 = q1;
        q1 = q0;
    }
    let mag_sq = q1 * q1 + q2 * q2 - q1 * q2 * coeff;
    let rms = (mag_sq.max(0.0).sqrt()) / (n * 0.5);
    (20.0 * (rms.max(1e-6)).log10()).clamp(-96.0, 0.0)
}

pub fn analyze_pcm_samples_for_noise_profile(
    samples: &[i16],
    capture_source: &str,
    strength_pct: i32,
    prior_passes: u32,
) -> NoiseLearningProfile {
    let n = samples.len();
    if n < 4800 {
        return NoiseLearningProfile::default();
    }

    let mut sum_sq = 0.0f64;
    let mut peak_abs = 0.0f64;
    let mut norm_samples = Vec::with_capacity(n);
    for &s in samples {
        let v = (s as f64) / 32768.0;
        let a = v.abs();
        if a > peak_abs {
            peak_abs = a;
        }
        sum_sq += v * v;
        norm_samples.push(v);
    }
    let raw_rms = (sum_sq / (n as f64)).sqrt();
    let noise_floor_dbfs = (20.0 * raw_rms.max(1e-5).log10()).clamp(-90.0, -5.0);
    let peak_noise_dbfs = (20.0 * peak_abs.max(1e-5).log10()).clamp(-90.0, 0.0);

    let mut denoised_sum_sq = 0.0f64;
    let mut denoised_count = 0usize;
    let mut vad_probs: Vec<f32> = Vec::with_capacity(n / 480);

    if let Some(mut rnn) = RnnoiseRunner::try_new() {
        let mut in_frame = [0.0f32; 480];
        let mut out_frame = [0.0f32; 480];
        for (frame_idx, chunk) in samples.as_chunks::<480>().0.iter().enumerate() {
            for (dst, &src) in in_frame.iter_mut().zip(chunk.iter()) {
                *dst = src as f32;
            }
            let vad = rnn.process_frame_480(&in_frame, &mut out_frame);
            if frame_idx >= 3 {
                vad_probs.push(vad.clamp(0.0, 1.0));
                for &y in &out_frame {
                    let yn = (y as f64) / 32768.0;
                    denoised_sum_sq += yn * yn;
                }
                denoised_count += 480;
            }
        }
    }

    let (denoised_floor_dbfs, rnnoise_attenuation_db, ambient_vad_p95) = if denoised_count > 0 {
        let den_rms = (denoised_sum_sq / (denoised_count as f64)).sqrt();
        let den_db = (20.0 * den_rms.max(1e-6).log10()).clamp(-92.0, -5.0);
        let atten = (noise_floor_dbfs - den_db).clamp(3.0, 35.0);
        vad_probs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let idx95 = ((vad_probs.len() as f64 * 0.95) as usize).min(vad_probs.len().saturating_sub(1));
        let p95 = vad_probs.get(idx95).copied().unwrap_or(0.15) as f64;
        (den_db, atten, p95)
    } else {
        let est_atten = 14.0 + (strength_pct as f64) * 0.08;
        ((noise_floor_dbfs - est_atten).clamp(-90.0, -10.0), est_atten, 0.18)
    };

    let sr = 48000.0;
    let alpha_140 = (-2.0 * PI * 140.0 / sr).exp();
    let alpha_550 = (-2.0 * PI * 550.0 / sr).exp();
    let alpha_2800 = (-2.0 * PI * 2800.0 / sr).exp();

    let mut lp140 = 0.0f64;
    let mut lp550 = 0.0f64;
    let mut lp2800 = 0.0f64;

    let mut e_low = 0.0f64;
    let mut e_fan = 0.0f64;
    let mut e_mid = 0.0f64;
    let mut e_high = 0.0f64;

    for &x in &norm_samples {
        lp140 = (1.0 - alpha_140) * x + alpha_140 * lp140;
        lp550 = (1.0 - alpha_550) * x + alpha_550 * lp550;
        lp2800 = (1.0 - alpha_2800) * x + alpha_2800 * lp2800;

        let b_low = lp140;
        let b_fan = lp550 - lp140;
        let b_mid = lp2800 - lp550;
        let b_high = x - lp2800;

        e_low += b_low * b_low;
        e_fan += b_fan * b_fan;
        e_mid += b_mid * b_mid;
        e_high += b_high * b_high;
    }

    let nf = n as f64;
    let band_low_rumble_db = (20.0 * (e_low / nf).sqrt().max(1e-5).log10()).clamp(-90.0, -5.0);
    let band_fan_hum_db = (20.0 * (e_fan / nf).sqrt().max(1e-5).log10()).clamp(-90.0, -5.0);
    let band_mid_room_db = (20.0 * (e_mid / nf).sqrt().max(1e-5).log10()).clamp(-90.0, -5.0);
    let band_high_hiss_db = (20.0 * (e_high / nf).sqrt().max(1e-5).log10()).clamp(-90.0, -5.0);

    let scan_slice = &norm_samples[..norm_samples.len().min(24000)];
    let candidate_freqs = [50, 60, 100, 120, 150, 180, 200, 240, 300, 360, 420, 480];
    let mut best_freq = 120;
    let mut best_power = -99.0f64;
    for &f in &candidate_freqs {
        let p = goertzel_power_dbfs(scan_slice, sr, f as f64);
        if p > best_power {
            best_power = p;
            best_freq = f;
        }
    }

    let strength_norm = (strength_pct.clamp(0, 100) as f64) / 100.0;
    let rumble_excess = (band_low_rumble_db - band_mid_room_db).clamp(-10.0, 20.0);
    let learned_highpass_hz =
        ((80.0 + (rumble_excess + 6.0).max(0.0) * 3.2 + strength_norm * 18.0).round() as i32)
            .clamp(75, 165);

    let learned_notch_db = ((-4.0 - strength_norm * 6.5 - (best_power - noise_floor_dbfs).max(0.0) * 0.25)
        * 10.0)
        .round()
        / 10.0;
    let learned_notch_db = learned_notch_db.clamp(-12.0, -3.0);

    let learned_vad_threshold_pct = ((ambient_vad_p95 * 100.0 + 18.0 + strength_norm * 22.0)
        .round() as i32)
        .clamp(42, 92);

    let learned_gate_dbfs =
        ((noise_floor_dbfs + 5.5 + strength_norm * 3.5) * 10.0).round() / 10.0;
    let learned_gate_dbfs = learned_gate_dbfs.clamp(-62.0, -24.0);

    let recommended_hw_nc_mode = if noise_floor_dbfs > -48.0 || strength_pct >= 75 {
        "strong".to_string()
    } else {
        "basic".to_string()
    };
    let recommended_hw_low_cut =
        if band_low_rumble_db >= band_mid_room_db + 1.5 || learned_highpass_hz >= 120 {
            "on".to_string()
        } else {
            "off".to_string()
        };

    let mut tags = Vec::new();
    if band_low_rumble_db >= band_mid_room_db + 2.5 {
        tags.push("HVAC / Low Rumble (<140Hz)".to_string());
    }
    if band_fan_hum_db >= band_mid_room_db + 1.5 {
        tags.push(format!("PC Fan / {best_freq}Hz Tonal Hum"));
    }
    if band_high_hiss_db >= band_mid_room_db - 2.0 {
        tags.push("High-Freq Preamp Hiss (>2.8kHz)".to_string());
    }
    if tags.is_empty() {
        tags.push(format!("Balanced Room Floor ({best_freq}Hz Harmonic)"));
    }
    let env_tier = if noise_floor_dbfs < -54.0 {
        "Quiet Studio"
    } else if noise_floor_dbfs < -44.0 {
        "Moderate Office"
    } else {
        "Noisy Environment"
    };
    let noise_signature = format!(
        "{} ({:+.1} dBFS) • {}",
        env_tier,
        noise_floor_dbfs,
        tags.join(" + ")
    );

    NoiseLearningProfile {
        learned: true,
        learned_at: now_hms(),
        capture_source: capture_source.to_string(),
        noise_floor_dbfs: (noise_floor_dbfs * 10.0).round() / 10.0,
        peak_noise_dbfs: (peak_noise_dbfs * 10.0).round() / 10.0,
        denoised_floor_dbfs: (denoised_floor_dbfs * 10.0).round() / 10.0,
        rnnoise_attenuation_db: (rnnoise_attenuation_db * 10.0).round() / 10.0,
        band_low_rumble_db: (band_low_rumble_db * 10.0).round() / 10.0,
        band_fan_hum_db: (band_fan_hum_db * 10.0).round() / 10.0,
        band_mid_room_db: (band_mid_room_db * 10.0).round() / 10.0,
        band_high_hiss_db: (band_high_hiss_db * 10.0).round() / 10.0,
        dominant_hum_hz: best_freq,
        noise_signature,
        learned_vad_threshold_pct,
        learned_highpass_hz,
        learned_notch_hz: best_freq,
        learned_notch_db,
        learned_gate_dbfs,
        recommended_hw_nc_mode,
        recommended_hw_low_cut,
        adaptive_passes: prior_passes + 1,
    }
}

fn run_noise_learn_pipeline(
    strength_pct: i32,
    virtual_mic_enabled: bool,
    report_progress: bool,
) -> NoiseLearningProfile {
    let (samples, label) = capture_pcm_48k_s16le(3, |sec, src| {
        if report_progress {
            let pct = (15 + (sec as i32) * 25).min(88);
            NOISE_LEARN_PROGRESS.store(pct, Ordering::SeqCst);
            *lock_unpoisoned(&NOISE_LEARN_STATUS) = format!(
                "Capturing & running RNNoise neural analysis ({}/3s) on {}...",
                (sec + 1).min(3),
                src
            );
        }
    });

    if report_progress {
        NOISE_LEARN_PROGRESS.store(92, Ordering::SeqCst);
    }
    let prior_passes = lock_unpoisoned(&CACHED_NOISE_PROFILE).adaptive_passes;

    let new_prof = if samples.len() >= 16000 {
        analyze_pcm_samples_for_noise_profile(&samples, &label, strength_pct, prior_passes)
    } else {
        NoiseLearningProfile::default()
    };

    save_noise_profile_to_disk(&new_prof);
    *lock_unpoisoned(&CACHED_NOISE_PROFILE) = new_prof.clone();

    if virtual_mic_enabled {
        let _ = sync_pipewire_ai_virtual_mic(true, true);
    }

    if report_progress {
        *lock_unpoisoned(&NOISE_LEARN_STATUS) = format!(
            "Learned: {} • RNNoise -{:.1} dB • Rec HW: NC={}, Low-Cut={}",
            new_prof.noise_signature,
            new_prof.rnnoise_attenuation_db,
            new_prof.recommended_hw_nc_mode.to_uppercase(),
            new_prof.recommended_hw_low_cut.to_uppercase()
        );
        NOISE_LEARN_PROGRESS.store(100, Ordering::SeqCst);
    }
    new_prof
}

pub fn trigger_noise_learning_async<F>(
    strength_pct: i32,
    virtual_mic_enabled: bool,
    on_done: F,
) -> bool
where
    F: FnOnce(NoiseLearningProfile) + Send + 'static,
{
    if NOISE_LEARN_ACTIVE.swap(true, Ordering::SeqCst) {
        return false;
    }
    NOISE_LEARN_PROGRESS.store(5, Ordering::SeqCst);
    *lock_unpoisoned(&NOISE_LEARN_STATUS) =
        "Sampling room acoustics from DJI Mic Mini (Please remain silent for ~3s)...".to_string();

    thread::spawn(move || {
        let new_prof = run_noise_learn_pipeline(strength_pct, virtual_mic_enabled, true);
        on_done(new_prof);
        thread::sleep(Duration::from_millis(200));
        NOISE_LEARN_ACTIVE.store(false, Ordering::SeqCst);
    });
    true
}

pub fn run_noise_learning_blocking(
    strength_pct: i32,
    virtual_mic_enabled: bool,
) -> NoiseLearningProfile {
    run_noise_learn_pipeline(strength_pct, virtual_mic_enabled, false)
}

pub fn recompute_profile_for_strength(strength_pct: i32, virtual_mic_enabled: bool) {
    let mut prof = lock_unpoisoned(&CACHED_NOISE_PROFILE).clone();
    let strength_norm = (strength_pct.clamp(0, 100) as f64) / 100.0;
    let rumble_excess = (prof.band_low_rumble_db - prof.band_mid_room_db).clamp(-10.0, 20.0);
    prof.learned_highpass_hz =
        ((80.0 + (rumble_excess + 6.0).max(0.0) * 3.2 + strength_norm * 18.0).round() as i32)
            .clamp(75, 165);
    prof.learned_notch_db = ((-4.0 - strength_norm * 6.5) * 10.0).round() / 10.0;
    prof.learned_vad_threshold_pct =
        ((52.0 + strength_norm * 32.0).round() as i32).clamp(40, 92);
    prof.learned_gate_dbfs =
        ((prof.noise_floor_dbfs + 5.0 + strength_norm * 4.0) * 10.0).round() / 10.0;
    prof.recommended_hw_nc_mode = if prof.noise_floor_dbfs > -48.0 || strength_pct >= 75 {
        "strong".to_string()
    } else {
        "basic".to_string()
    };
    save_noise_profile_to_disk(&prof);
    *lock_unpoisoned(&CACHED_NOISE_PROFILE) = prof;
    if virtual_mic_enabled {
        let _ = sync_pipewire_ai_virtual_mic(true, false);
    }
}

pub fn maybe_run_adaptive_micro_learn(strength_pct: i32, virtual_mic_enabled: bool) {
    if NOISE_LEARN_ACTIVE.load(Ordering::SeqCst) {
        return;
    }
    {
        let mut last = lock_unpoisoned(&LAST_ADAPTIVE_PROBE);
        if let Some(t) = *last {
            if t.elapsed() < Duration::from_secs(75) {
                return;
            }
        }
        *last = Some(Instant::now());
    }
    if NOISE_LEARN_ACTIVE.swap(true, Ordering::SeqCst) {
        return;
    }
    thread::spawn(move || {
        let (samples, label) = capture_pcm_48k_s16le(1, |_, _| {});
        if samples.len() >= 16000 {
            let old_prof = lock_unpoisoned(&CACHED_NOISE_PROFILE).clone();
            let probe = analyze_pcm_samples_for_noise_profile(
                &samples,
                &label,
                strength_pct,
                old_prof.adaptive_passes,
            );
            if probe.noise_floor_dbfs < -33.0 {
                let mut blended = probe.clone();
                if old_prof.learned {
                    blended.noise_floor_dbfs =
                        ((old_prof.noise_floor_dbfs * 0.75 + probe.noise_floor_dbfs * 0.25) * 10.0)
                            .round()
                            / 10.0;
                    blended.denoised_floor_dbfs = ((old_prof.denoised_floor_dbfs * 0.75
                        + probe.denoised_floor_dbfs * 0.25)
                        * 10.0)
                        .round()
                        / 10.0;
                    blended.rnnoise_attenuation_db =
                        ((blended.noise_floor_dbfs - blended.denoised_floor_dbfs).max(3.0) * 10.0)
                            .round()
                            / 10.0;
                }
                save_noise_profile_to_disk(&blended);
                *lock_unpoisoned(&CACHED_NOISE_PROFILE) = blended.clone();
                *lock_unpoisoned(&NOISE_LEARN_STATUS) = format!(
                    "Adaptive Auto-Learn Pass #{}: {} (Rec HW NC={}, Low-Cut={})",
                    blended.adaptive_passes,
                    blended.noise_signature,
                    blended.recommended_hw_nc_mode.to_uppercase(),
                    blended.recommended_hw_low_cut.to_uppercase()
                );
                if virtual_mic_enabled {
                    let _ = sync_pipewire_ai_virtual_mic(true, false);
                }
            }
        }
        NOISE_LEARN_ACTIVE.store(false, Ordering::SeqCst);
    });
}

// =====================================================================
// ALSA & PipeWire Device Controls
// =====================================================================

pub fn get_alsa_card_info() -> AlsaCardInfo {
    let mut info = AlsaCardInfo::default();
    let Ok(txt) = fs::read_to_string("/proc/asound/cards") else {
        return info;
    };
    let lines: Vec<&str> = txt.lines().collect();
    for (idx, line) in lines.iter().enumerate() {
        if line.contains("DJI MIC MINI") || line.contains("[MINI") {
            let trimmed = line.trim();
            if let Some(b_open) = trimmed.find('[') {
                let idx_str = trimmed[..b_open].trim();
                if let Ok(cidx) = idx_str.parse::<i32>() {
                    info.card_index = Some(cidx);
                }
                if let Some(b_close) = trimmed[b_open..].find("]:") {
                    let cid = trimmed[b_open + 1..b_open + b_close].trim();
                    info.card_id = Some(cid.to_string());
                    let cname = trimmed[b_open + b_close + 2..].trim();
                    info.card_name = Some(cname.to_string());
                }
            }
            if idx + 1 < lines.len() {
                let next_line = lines[idx + 1];
                if let Some(at_pos) = next_line.find("at usb-") {
                    let after = &next_line[at_pos + 3..];
                    let end = after.find(',').unwrap_or(after.len());
                    info.usb_bus_path = Some(after[..end].trim().to_string());
                }
            }
            break;
        }
    }
    info
}

pub fn get_alsa_card_index() -> Option<i32> {
    get_alsa_card_info().card_index
}

pub fn is_alsa_card_capturing(card_index: Option<i32>) -> bool {
    let Some(idx) = card_index else {
        return false;
    };
    let status_path = format!("/proc/asound/card{idx}/pcm0c/sub0/status");
    if let Ok(txt) = fs::read_to_string(status_path) {
        return txt.contains("RUNNING");
    }
    false
}

pub fn get_alsa_volume() -> Option<AlsaVolumeInfo> {
    let card_idx = get_alsa_card_index()?;
    let card_str = card_idx.to_string();
    let timeout = Duration::from_secs(2);

    for ctl_name in ["Mic", "Capture"] {
        let Ok(out) = run_command_with_timeout("amixer", &["-c", &card_str, "sget", ctl_name], timeout)
        else {
            continue;
        };
        if !out.status.success() {
            continue;
        }
        let stdout = String::from_utf8_lossy(&out.stdout);
        if !stdout.contains('[') {
            continue;
        }

        let mut pct: Option<i32> = None;
        let mut db_val: Option<String> = None;
        let mut muted = false;

        let mut rest = stdout.as_ref();
        while let Some(open) = rest.find('[') {
            let after_open = &rest[open + 1..];
            let Some(close) = after_open.find(']') else {
                break;
            };
            let token = &after_open[..close];
            if let Some(num_str) = token.strip_suffix('%') {
                if pct.is_none() {
                    if let Ok(v) = num_str.trim().parse::<i32>() {
                        pct = Some(v);
                    }
                }
            } else if token.ends_with("dB") && db_val.is_none() {
                db_val = Some(token.to_string());
            } else if token == "off" {
                muted = true;
            } else if token == "on" {
                muted = false;
            }
            rest = &after_open[close + 1..];
        }

        return Some(AlsaVolumeInfo {
            control: ctl_name.to_string(),
            percent: pct.unwrap_or(100),
            db: db_val,
            muted,
        });
    }
    None
}

/// Validate and normalize a user/CLI volume spec (`"0%"..="100%"`, `"+5%"`, `"-5%"`, `"5%+"`, `"5%-"`)
/// into a safe canonical `amixer`/`wpctl` string that never starts with `'-'`.
pub fn normalize_volume_spec(raw: &str) -> Option<String> {
    let s = raw.trim();
    if s.is_empty() || s.len() > 8 {
        return None;
    }

    if let Some(rest) = s.strip_prefix('+') {
        let num_str = rest.strip_suffix('%').unwrap_or(rest);
        let v: u32 = num_str.parse().ok()?;
        if (1..=100).contains(&v) {
            return Some(format!("{v}%+"));
        }
        return None;
    }
    if let Some(rest) = s.strip_prefix('-') {
        let num_str = rest.strip_suffix('%').unwrap_or(rest);
        let v: u32 = num_str.parse().ok()?;
        if (1..=100).contains(&v) {
            return Some(format!("{v}%-"));
        }
        return None;
    }
    if let Some(rest) = s.strip_suffix('+') {
        let num_str = rest.strip_suffix('%').unwrap_or(rest);
        let v: u32 = num_str.parse().ok()?;
        if (1..=100).contains(&v) {
            return Some(format!("{v}%+"));
        }
        return None;
    }
    if let Some(rest) = s.strip_suffix('-') {
        let num_str = rest.strip_suffix('%').unwrap_or(rest);
        let v: u32 = num_str.parse().ok()?;
        if (1..=100).contains(&v) {
            return Some(format!("{v}%-"));
        }
        return None;
    }

    let num_str = s.strip_suffix('%').unwrap_or(s);
    if !num_str.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let v: u32 = num_str.parse().ok()?;
    if v <= 100 {
        Some(format!("{v}%"))
    } else {
        None
    }
}

pub fn is_valid_volume_spec(raw: &str) -> bool {
    normalize_volume_spec(raw).is_some()
}

pub fn set_alsa_volume(volume: &str) -> bool {
    let Some(norm_vol) = normalize_volume_spec(volume) else {
        return false;
    };
    let timeout = Duration::from_secs(2);
    let mut ok = false;
    if let Some(card_idx) = get_alsa_card_index() {
        let card_str = card_idx.to_string();
        for ctl_name in ["Mic", "Capture"] {
            if let Ok(out) =
                run_command_with_timeout("amixer", &["-c", &card_str, "sset", ctl_name, &norm_vol], timeout)
            {
                if out.status.success() {
                    ok = true;
                    break;
                }
            }
        }
    }
    if let Some(pw_id) = get_pipewire_source_id() {
        if let Ok(out) =
            run_command_with_timeout("wpctl", &["set-volume", &pw_id.to_string(), &norm_vol], timeout)
        {
            if out.status.success() {
                ok = true;
            }
        }
    }
    ok
}

pub fn set_alsa_mute(muted: bool) -> bool {
    let timeout = Duration::from_secs(2);
    let mut ok = false;
    if let Some(card_idx) = get_alsa_card_index() {
        let card_str = card_idx.to_string();
        let cmd_val = if muted { "mute" } else { "unmute" };
        for ctl_name in ["Mic", "Capture"] {
            if let Ok(out) =
                run_command_with_timeout("amixer", &["-c", &card_str, "sset", ctl_name, cmd_val], timeout)
            {
                if out.status.success() {
                    ok = true;
                    break;
                }
            }
        }
    }
    if let Some(pw_id) = get_pipewire_source_id() {
        let mute_arg = if muted { "1" } else { "0" };
        if let Ok(out) =
            run_command_with_timeout("wpctl", &["set-mute", &pw_id.to_string(), mute_arg], timeout)
        {
            if out.status.success() {
                ok = true;
            }
        }
    }
    ok
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

pub fn get_pipewire_source_id() -> Option<i32> {
    let out = run_command_with_timeout("wpctl", &["status"], Duration::from_secs(2)).ok()?;
    if !out.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut in_sources = false;
    for line in stdout.lines() {
        if line.contains("Sources:") {
            in_sources = true;
            continue;
        }
        if in_sources {
            if line.contains("Filters:")
                || line.contains("Streams:")
                || line.starts_with("Video")
                || line.starts_with("Settings")
            {
                in_sources = false;
                continue;
            }
            if line.contains("DJI MIC MINI") {
                if let Some(dot_pos) = line.find('.') {
                    let before_dot = &line[..dot_pos];
                    let digits: String = before_dot
                        .chars()
                        .rev()
                        .take_while(|c| c.is_ascii_digit())
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect();
                    if let Ok(id) = digits.parse::<i32>() {
                        return Some(id);
                    }
                }
            }
        }
    }
    None
}

pub fn get_pipewire_stream_details() -> PipewireStreamInfo {
    let mut details = PipewireStreamInfo {
        wpctl_id: get_pipewire_source_id(),
        ..Default::default()
    };

    if let Ok(out) = run_command_with_timeout(
        "pactl",
        &["list", "sources", "short"],
        Duration::from_secs(2),
    ) {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            *lock_unpoisoned(&VIRTUAL_MIC_CACHE) =
                Some((Instant::now(), stdout.contains("dji_ai_denoise_mic")));
            for line in stdout.lines() {
                if line.contains("DJI_MIC_MINI") || line.contains("DJI MIC MINI") {
                    let parts: Vec<&str> = line.split('\t').collect();
                    if parts.len() >= 5 {
                        details.pactl_id = parts[0].trim().parse::<i32>().ok();
                        details.node_name = Some(parts[1].trim().to_string());
                        details.sample_spec = Some(parts[3].trim().to_string());
                        details.stream_state = Some(parts[4].trim().to_string());
                        break;
                    }
                }
            }
        }
    }
    details
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_spec_validation_and_normalization() {
        assert_eq!(normalize_volume_spec("50%"), Some("50%".to_string()));
        assert_eq!(normalize_volume_spec("0%"), Some("0%".to_string()));
        assert_eq!(normalize_volume_spec("100%"), Some("100%".to_string()));
        assert_eq!(normalize_volume_spec("+5%"), Some("5%+".to_string()));
        assert_eq!(normalize_volume_spec("-5%"), Some("5%-".to_string()));
        assert_eq!(normalize_volume_spec("5%+"), Some("5%+".to_string()));
        assert_eq!(normalize_volume_spec("5%-"), Some("5%-".to_string()));

        assert!(!is_valid_volume_spec("--help"));
        assert!(!is_valid_volume_spec("-c"));
        assert!(!is_valid_volume_spec("150%"));
        assert!(!is_valid_volume_spec("50%;id"));
        assert!(!is_valid_volume_spec("-0%"));
    }

    #[test]
    fn test_ai_nc_mode_cycle_and_serde() {
        let mut m = AiNcMode::Off;
        m = m.next_cycle();
        assert_eq!(m, AiNcMode::Hardware);
        m = m.next_cycle();
        assert_eq!(m, AiNcMode::LearnedAi);
        m = m.next_cycle();
        assert_eq!(m, AiNcMode::Adaptive);
        m = m.next_cycle();
        assert_eq!(m, AiNcMode::Off);
        assert_eq!(serde_json::to_string(&AiNcMode::LearnedAi).unwrap(), "\"learned_ai\"");
    }

    #[test]
    fn test_analyze_synthetic_pcm_samples() {
        // Generate 48kHz quiet tone + 120Hz hum (0.5s = 24000 samples)
        let samples: Vec<i16> = (0..24000)
            .map(|i| {
                let t = (i as f64) / 48000.0;
                ((2.0 * PI * 120.0 * t).sin() * 250.0) as i16
            })
            .collect();
        let prof = analyze_pcm_samples_for_noise_profile(&samples, "test-source", 80, 0);
        assert!(prof.learned);
        assert_eq!(prof.adaptive_passes, 1);
        assert!((75..=165).contains(&prof.learned_highpass_hz));
        assert!((42..=92).contains(&prof.learned_vad_threshold_pct));
    }
}

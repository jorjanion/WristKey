#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use wristkey_core::crypto::IrkResolver;
use wristkey_core::dpapi::{CredentialVault, UserCredentials};
use wristkey_core::ipc::IpcServer;
use wristkey_core::lock::lock_workstation;
use wristkey_core::registry::{read_paired_bluetooth_keys, PairedBluetoothKey};
use wristkey_core::sas::LockScreenController;
use wristkey_core::scanner::BleScanner;

#[derive(serde::Serialize, Clone, Debug)]
pub struct DiscoveredDeviceDto {
    pub mac: String,
    pub name: String,
    pub rssi: i16,
    pub on_wrist: bool,
    pub is_unlocked: bool,
    pub frame_type: String,
    pub is_watch_candidate: bool,
    pub is_confirmed_watch: bool,
    pub device_type: String,
    pub last_seen_ms_ago: u64,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct ProximityReadingDto {
    pub current_mac: String,
    pub device_name: String,
    pub is_confirmed_watch: bool,
    pub on_wrist: bool,
    pub is_unlocked: bool,
    pub rssi: f32,
    pub smoothed_rssi: f32,
    pub distance_meters: f32,
    pub is_near: bool,
    pub is_lost: bool,
    pub seconds_since_last_packet: f32,
    pub status_text: String,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub download_url: String,
    pub release_notes: String,
}

struct DeviceEntry {
    device: DiscoveredDeviceDto,
    last_seen: Instant,
    smoothed_rssi_f32: f32,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct EngineSettings {
    pub auto_lock_enabled: bool,
    pub auto_unlock_enabled: bool,
    pub require_on_wrist: bool,
    pub lock_threshold_dbm: i32,
    pub unlock_threshold_dbm: i32,
    pub tracked_mac: Option<String>,
}

impl Default for EngineSettings {
    fn default() -> Self {
        Self {
            auto_lock_enabled: true,
            auto_unlock_enabled: true,
            require_on_wrist: true,
            lock_threshold_dbm: -74,
            unlock_threshold_dbm: -58,
            tracked_mac: None,
        }
    }
}

fn get_settings_path() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let p = PathBuf::from(appdata).join("WristKey");
            let _ = std::fs::create_dir_all(&p);
            return Some(p.join("settings.json"));
        }
    }
    None
}

fn load_settings() -> EngineSettings {
    if let Some(path) = get_settings_path() {
        if let Ok(data) = std::fs::read_to_string(path) {
            if let Ok(s) = serde_json::from_str::<EngineSettings>(&data) {
                return s;
            }
        }
    }
    EngineSettings::default()
}

fn save_settings(settings: &EngineSettings) {
    if let Some(path) = get_settings_path() {
        if let Ok(data) = serde_json::to_string_pretty(settings) {
            let _ = std::fs::write(path, data);
        }
    }
}

static ENGINE_SETTINGS: std::sync::LazyLock<Arc<Mutex<EngineSettings>>> =
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(load_settings())));

static IS_LOCKED: AtomicBool = AtomicBool::new(false);

static DEVICES: std::sync::LazyLock<Arc<Mutex<HashMap<String, DeviceEntry>>>> =
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

#[tauri::command]
fn get_discovered_devices() -> Vec<DiscoveredDeviceDto> {
    let now = Instant::now();
    let mut results = Vec::new();

    if let Ok(mut lock) = DEVICES.lock() {
        // Drop devices not seen in the last 8 seconds
        lock.retain(|_, entry| now.duration_since(entry.last_seen).as_secs() < 8);

        for entry in lock.values() {
            // ONLY return Apple Watches detected on wrist in desk/room proximity (RSSI >= -76 dBm)
            // Distant devices from other rooms (e.g. father's watch in another room at -81 dBm) are excluded.
            if entry.device.on_wrist && entry.device.rssi >= -76 {
                let mut dev = entry.device.clone();
                dev.last_seen_ms_ago = now.duration_since(entry.last_seen).as_millis() as u64;
                results.push(dev);
            }
        }
    }

    // Sort order:
    // 1. Confirmed Apple Watches FIRST
    // 2. Highest RSSI (closest to desk) SECOND
    results.sort_by(|a, b| {
        b.is_confirmed_watch
            .cmp(&a.is_confirmed_watch)
            .then_with(|| b.rssi.cmp(&a.rssi))
    });

    // The user explicitly requires that ONLY their Apple Watch appear in the scan:
    // Strictly isolate the user's Apple Watch (the closest one at the desk).
    if !results.is_empty() {
        results.truncate(1);
    }

    results
}

#[tauri::command]
fn get_live_proximity(mac: Option<String>) -> ProximityReadingDto {
    let now = Instant::now();
    if let Ok(lock) = DEVICES.lock() {
        // Target tracking with seamless RPA MAC rotation handling:
        let target = if let Some(ref m) = mac {
            if let Some(entry) = lock.get(m) {
                let elapsed = now.duration_since(entry.last_seen).as_secs_f32();
                // Allow RPA rotation handoff if the old MAC has been completely silent for >= 15 seconds
                // AND a replacement confirmed Apple Watch is present at desk proximity (>= -65 dBm) on-wrist.
                // Under NO circumstances should it hop between phone and watch!
                if elapsed >= 15.0 {
                    let fresh_watch = lock
                        .values()
                        .filter(|e| {
                            e.device.mac != *m
                                && e.device.is_confirmed_watch
                                && e.device.on_wrist
                                && now.duration_since(e.last_seen).as_secs_f32() < 2.5
                                && e.device.rssi >= -65
                        })
                        .max_by(|a, b| a.device.rssi.cmp(&b.device.rssi));

                    Some(fresh_watch.unwrap_or(entry))
                } else {
                    Some(entry)
                }
            } else {
                let mut watches: Vec<_> = lock
                    .values()
                    .filter(|e| e.device.on_wrist && e.device.rssi >= -76)
                    .collect();
                watches.sort_by(|a, b| {
                    b.device.is_confirmed_watch
                        .cmp(&a.device.is_confirmed_watch)
                        .then_with(|| b.device.rssi.cmp(&a.device.rssi))
                });
                watches.first().copied()
            }
        } else {
            let mut watches: Vec<_> = lock
                .values()
                .filter(|e| e.device.on_wrist && e.device.rssi >= -76)
                .collect();
            watches.sort_by(|a, b| {
                b.device.is_confirmed_watch
                    .cmp(&a.device.is_confirmed_watch)
                    .then_with(|| b.device.rssi.cmp(&a.device.rssi))
            });
            watches.first().copied()
        };

        if let Some(entry) = target {
            let elapsed_secs = now.duration_since(entry.last_seen).as_secs_f32();

            // RESPONSIVE SIGNAL FADE & SILENCE DECAY:
            // 0s to 2.5s: Steady smoothed RSSI during active Bluetooth transmissions.
            // > 2.5s: If user is walking away or puts body between watch & PC, fade at 5.0 dB/s.
            // > 10.0s: Completely out of range or silent on charger -> triggers loss.
            let effective_rssi = if elapsed_secs > 2.5 {
                let fade = (elapsed_secs - 2.5) * 5.0;
                (entry.smoothed_rssi_f32 - fade).max(-88.0)
            } else {
                entry.smoothed_rssi_f32
            };

            // Accurate Log-Distance Path Loss Model:
            // Reference at 1m is -58.0 dBm for Apple Watch on wrist; path loss exponent n = 2.4.
            let ratio = (-58.0 - effective_rssi) / (10.0 * 2.4);
            let distance_meters = (10.0_f32).powf(ratio).clamp(0.3, 8.0);

            let is_lost = elapsed_secs > 10.0 || effective_rssi <= -78.0;
            let is_near = effective_rssi >= -62.0 && !is_lost;

            // ON-WRIST REAL-TIME VALIDITY:
            // 1. If device is lost or silent > 3.5s, it is NOT actively communicating on-wrist.
            // 2. If the watch explicitly transmitted an off-wrist / charging frame (Action 0x05, 0x03, or is_locked),
            //    it is immediately recognized as off-wrist.
            let is_active_on_wrist = entry.device.on_wrist && elapsed_secs <= 3.5;
            let is_active_unlocked = entry.device.is_unlocked && elapsed_secs <= 3.5;

            let (display_name, display_on_wrist, display_unlocked) = if is_lost {
                ("Apple Watch (Sin señal / Fuera de alcance)".to_string(), false, false)
            } else if elapsed_secs > 3.5 {
                ("Apple Watch (Sin señal reciente / Fuera de muñeca)".to_string(), false, false)
            } else if !entry.device.on_wrist {
                ("Apple Watch (Bloqueado / Fuera de muñeca / En cargador)".to_string(), false, false)
            } else {
                (entry.device.name.clone(), is_active_on_wrist, is_active_unlocked)
            };

            let status_text = if is_lost {
                "LEJOS / SIN SEÑAL (Zona de bloqueo)".to_string()
            } else if !display_on_wrist {
                "FUERA DE MUÑECA (Zona de bloqueo)".to_string()
            } else if is_near {
                "CERCA (En tu escritorio / Desbloqueado)".to_string()
            } else if effective_rssi <= -72.0 {
                "ALEJÁNDOTE (Próximo al bloqueo)".to_string()
            } else {
                "ZONA MEDIA (En la habitación)".to_string()
            };

            return ProximityReadingDto {
                current_mac: entry.device.mac.clone(),
                device_name: display_name,
                is_confirmed_watch: entry.device.is_confirmed_watch,
                on_wrist: display_on_wrist,
                is_unlocked: display_unlocked,
                rssi: effective_rssi,
                smoothed_rssi: effective_rssi,
                distance_meters,
                is_near,
                is_lost,
                seconds_since_last_packet: elapsed_secs,
                status_text,
            };
        }
    }

    ProximityReadingDto {
        current_mac: "".to_string(),
        device_name: "Buscando tu Apple Watch...".to_string(),
        is_confirmed_watch: false,
        on_wrist: false,
        is_unlocked: false,
        rssi: -88.0,
        smoothed_rssi: -88.0,
        distance_meters: 8.0,
        is_near: false,
        is_lost: true,
        seconds_since_last_packet: 12.0,
        status_text: "SIN SEÑAL (Coloca tu reloj en la muñeca)".to_string(),
    }
}

#[tauri::command]
fn get_device_rssi(mac: String) -> Option<i16> {
    if let Ok(lock) = DEVICES.lock() {
        lock.get(&mac).map(|e| e.device.rssi)
    } else {
        None
    }
}

#[tauri::command]
fn is_running_as_admin() -> bool {
    wristkey_core::registry::is_elevated()
}

#[tauri::command]
fn relaunch_as_admin() -> Result<(), String> {
    if wristkey_core::registry::is_elevated() {
        return Ok(());
    }

    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_str = current_exe.to_string_lossy().to_string();

    let mut cmd = std::process::Command::new("powershell.exe");
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let status = cmd
        .args(&[
            "-WindowStyle",
            "Hidden",
            "-Command",
            &format!("Start-Process -FilePath '{}' -Verb RunAs", exe_str),
        ])
        .status()
        .map_err(|e| format!("Error launching UAC prompt: {}", e))?;

    if status.success() {
        std::process::exit(0);
    } else {
        Err("El usuario canceló la elevación de permisos UAC".to_string())
    }
}

#[tauri::command]
async fn check_for_updates() -> Result<Option<UpdateInfo>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let current_version = env!("CARGO_PKG_VERSION");

        let mut cmd = std::process::Command::new("curl.exe");
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let output = cmd
            .args(&[
                "-s",
                "-H",
                "User-Agent: WristKey",
                "--max-time",
                "4",
                "https://api.github.com/repos/jorjanion/WristKey/releases/latest",
            ])
            .output()
            .map_err(|e| format!("Failed to check for updates: {}", e))?;

        if !output.status.success() {
            return Ok(None);
        }

        let body = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(_) => return Ok(None),
        };

        let tag = json.get("tag_name").and_then(|v| v.as_str()).unwrap_or("");
        let latest_clean = tag.trim_start_matches('v');

        if !latest_clean.is_empty() && latest_clean != current_version {
            let mut download_url = json
                .get("html_url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if let Some(assets) = json.get("assets").and_then(|v| v.as_array()) {
                for asset in assets {
                    if let Some(name) = asset.get("name").and_then(|v| v.as_str()) {
                        if name.ends_with(".exe") {
                            if let Some(url) = asset.get("browser_download_url").and_then(|v| v.as_str()) {
                                download_url = url.to_string();
                                break;
                            }
                        }
                    }
                }
            }

            let notes = json.get("body").and_then(|v| v.as_str()).unwrap_or("").to_string();

            return Ok(Some(UpdateInfo {
                current_version: current_version.to_string(),
                latest_version: tag.to_string(),
                download_url,
                release_notes: notes,
            }));
        }

        Ok(None)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn download_and_install_update(download_url: String) -> Result<(), String> {
    std::thread::spawn(move || {
        let temp_dir = std::env::temp_dir();
        let target_exe = temp_dir.join("WristKey-Update.exe");
        let target_path = target_exe.to_string_lossy().to_string();

        let mut cmd = std::process::Command::new("curl.exe");
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let status = cmd
            .args(&["-L", "-s", "-o", &target_path, &download_url])
            .status();

        if let Ok(s) = status {
            if s.success() {
                let mut run_cmd = std::process::Command::new(&target_exe);
                #[cfg(windows)]
                run_cmd.creation_flags(CREATE_NO_WINDOW);
                let _ = run_cmd.spawn();
                std::process::exit(0);
            }
        }
    });

    Ok(())
}

#[tauri::command]
fn query_registry_keys() -> Result<Vec<PairedBluetoothKey>, String> {
    read_paired_bluetooth_keys().map_err(|e| e.to_string())
}

#[tauri::command]
fn verify_irk(irk: String) -> Result<bool, String> {
    IrkResolver::from_hex(&irk)
        .map(|_| true)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn save_credentials(domain: String, username: String, password: String) -> Result<(), String> {
    let creds = UserCredentials::new(domain, username, password);
    let vault = CredentialVault::new_default();
    vault.save(&creds).map_err(|e| e.to_string())
}

#[tauri::command]
fn test_vault() -> Result<UserCredentials, String> {
    let vault = CredentialVault::new_default();
    vault.load().map_err(|e| e.to_string())
}

#[tauri::command]
fn test_workstation_lock() -> Result<(), String> {
    lock_workstation().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_autostart_enabled() -> bool {
    #[cfg(windows)]
    {
        use std::process::Command;
        let mut cmd = Command::new("reg.exe");
        cmd.creation_flags(CREATE_NO_WINDOW);
        let output = cmd
            .args(&["query", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run", "/v", "WristKey"])
            .output();
        if let Ok(out) = output {
            return out.status.success();
        }
    }
    false
}

#[tauri::command]
fn set_autostart_enabled(enabled: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::process::Command;
        if enabled {
            if let Ok(exe_path) = std::env::current_exe() {
                let path_str = exe_path.to_string_lossy().to_string();
                let mut cmd = Command::new("reg.exe");
                cmd.creation_flags(CREATE_NO_WINDOW);
                let status = cmd
                    .args(&[
                        "add",
                        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                        "/v",
                        "WristKey",
                        "/t",
                        "REG_SZ",
                        "/d",
                        &format!("\"{}\"", path_str),
                        "/f",
                    ])
                    .status();
                if let Ok(s) = status {
                    if s.success() {
                        return Ok(());
                    }
                }
            }
            return Err("No se pudo registrar en el inicio de Windows".to_string());
        } else {
            let mut cmd = Command::new("reg.exe");
            cmd.creation_flags(CREATE_NO_WINDOW);
            let _ = cmd
                .args(&[
                    "delete",
                    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                    "/v",
                    "WristKey",
                    "/f",
                ])
                .status();
            return Ok(());
        }
    }
    #[cfg(not(windows))]
    Ok(())
}

fn start_background_scanner() {
    std::thread::spawn(|| {
        let mut scanner = BleScanner::new();
        match scanner.start() {
            Ok(rx) => {
                log::info!("Live WinRT BLE scanner started for UI");
                while let Ok(event) = rx.recv() {
                    if let Some(frame) = event.apple_frame {
                        // Filter out non-watch Apple accessories: AirTags / Find My trackers, AirPods
                        if frame.message_type == wristkey_core::continuity::TYPE_FIND_MY
                            || frame.message_type == 0x12
                            || frame.message_type == wristkey_core::continuity::TYPE_PROXIMITY_PAIRING
                        {
                            continue;
                        }

                        let mac_str = format!(
                            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                            (event.mac >> 40) & 0xFF,
                            (event.mac >> 32) & 0xFF,
                            (event.mac >> 24) & 0xFF,
                            (event.mac >> 16) & 0xFF,
                            (event.mac >> 8) & 0xFF,
                            event.mac & 0xFF
                        );

                        // STRICT APPLE WATCH ADMISSION:
                        // Reject iPhones, iPads, Macs, and accessories.
                        // Only confirmed Apple Watch frames are admitted.
                        // If it's not a confirmed watch, we ONLY accept it if mac_str was ALREADY confirmed
                        // as our Apple Watch and is now reporting locked/off-wrist state (is_locked == true).
                        let was_already_watch = if let Ok(lock) = DEVICES.lock() {
                            lock.get(&mac_str).map_or(false, |e| e.device.is_confirmed_watch)
                        } else {
                            false
                        };

                        if !frame.is_confirmed_apple_watch() && !(was_already_watch && frame.is_locked) {
                            continue;
                        }

                        if let Ok(mut lock) = DEVICES.lock() {
                            let is_on_wrist = frame.is_on_wrist_and_unlocked;
                            let is_confirmed_watch = was_already_watch || frame.is_confirmed_apple_watch();

                            let name = if is_on_wrist {
                                "Apple Watch (En tu muñeca y desbloqueado)".to_string()
                            } else {
                                "Apple Watch (Bloqueado / Fuera de muñeca / En cargador)".to_string()
                            };

                            let frame_desc = format!("{} (0x{:02X})", frame.message_type_name, frame.message_type);

                            // HIGH-PRECISION ADAPTIVE RSSI SMOOTHING:
                            let raw_f = event.rssi as f32;
                            let smoothed_rssi_f32 = if let Some(existing) = lock.get(&mac_str) {
                                let prev = existing.smoothed_rssi_f32;
                                let diff = (raw_f - prev).abs();

                                // Outlier clamp for physical sanity (> 30 dB spikes due to RF collision)
                                let clamped_raw = if (raw_f - prev) > 30.0 {
                                    prev + 30.0
                                } else if (prev - raw_f) > 30.0 {
                                    prev - 30.0
                                } else {
                                    raw_f
                                };

                                // Asymmetric, highly-responsive tracking:
                                // Signal drops (walking away, turning back, putting body between PC & watch):
                                // High alpha (0.75) gives instant real-time response!
                                // Signal strengthens (approaching desk): Alpha 0.45 for smooth approach.
                                // Minimal noise (< 3 dBm): Alpha 0.20 for rock-solid stability at desk.
                                let alpha = if diff < 3.0 {
                                    0.20
                                } else if raw_f < prev {
                                    0.75
                                } else {
                                    0.45
                                };

                                (prev * (1.0 - alpha)) + (clamped_raw * alpha)
                            } else {
                                raw_f
                            };

                            let dto = DiscoveredDeviceDto {
                                mac: mac_str.clone(),
                                name,
                                rssi: smoothed_rssi_f32.round() as i16,
                                on_wrist: is_on_wrist,
                                is_unlocked: !frame.is_locked,
                                frame_type: frame_desc,
                                is_watch_candidate: true,
                                is_confirmed_watch,
                                device_type: if is_confirmed_watch {
                                    "apple_watch".to_string()
                                } else {
                                    "apple_candidate".to_string()
                                },
                                last_seen_ms_ago: 0,
                            };

                            lock.insert(
                                mac_str,
                                DeviceEntry {
                                    device: dto,
                                    last_seen: Instant::now(),
                                    smoothed_rssi_f32,
                                },
                            );
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to start BLE scanner: {:?}", e);
            }
        }
    });
}

#[tauri::command]
fn get_engine_settings() -> EngineSettings {
    if let Ok(guard) = ENGINE_SETTINGS.lock() {
        guard.clone()
    } else {
        EngineSettings::default()
    }
}

#[tauri::command]
fn update_engine_settings(new_settings: EngineSettings) -> Result<(), String> {
    if let Ok(mut guard) = ENGINE_SETTINGS.lock() {
        *guard = new_settings.clone();
        save_settings(&new_settings);
    }
    Ok(())
}

fn start_native_proximity_engine() {
    std::thread::spawn(|| {
        log::info!("WristKey Native Proximity Lock/Unlock Engine started");
        let startup_time = Instant::now();
        let mut session_armed = false;
        let mut consecutive_away_ticks: u32 = 0;
        let mut consecutive_near_ticks: u32 = 0;
        let mut lock_cooldown_until = Instant::now();

        loop {
            std::thread::sleep(std::time::Duration::from_millis(400));
            let now = Instant::now();

            let settings = if let Ok(s) = ENGINE_SETTINGS.lock() {
                s.clone()
            } else {
                continue;
            };

            let reading = get_live_proximity(settings.tracked_mac.clone());
            let currently_locked = IS_LOCKED.load(Ordering::SeqCst);

            // ARMING LOGIC:
            // Protection ONLY arms when the user's watch is confirmed in desk proximity (< 2m, on-wrist).
            // If the watch is not connected or in another room on startup, we do NOT lock the PC!
            if !session_armed {
                if reading.is_near && (!settings.require_on_wrist || (reading.is_confirmed_watch && reading.on_wrist && reading.is_unlocked)) {
                    session_armed = true;
                    log::info!(
                        "[NATIVE ENGINE] Session ARMED: Apple Watch confirmed at desk ({:.1} dBm). Proximity protection active!",
                        reading.rssi
                    );
                }
            }

            // EVALUATE AUTO-LOCK:
            // Must be enabled, not currently locked, beyond startup grace period (12s),
            // past any lock cooldown, and the session must have been armed at least once!
            if settings.auto_lock_enabled
                && !currently_locked
                && session_armed
                && now.duration_since(startup_time).as_secs() >= 12
                && now >= lock_cooldown_until
            {
                // Criteria for being "Away":
                // 1. Device is lost (> 18s of silence / out of range)
                // 2. Smoothed RSSI has fallen below lock_threshold_dbm (e.g. <= -74 dBm)
                // 3. (If require_on_wrist) watch has been removed from wrist or locked
                let is_away = reading.is_lost
                    || reading.rssi <= settings.lock_threshold_dbm as f32
                    || (settings.require_on_wrist && (!reading.on_wrist || !reading.is_unlocked));

                if is_away {
                    consecutive_away_ticks += 1;
                    // Require 7 consecutive ticks (~2.8 seconds) of confirmed departure
                    // to completely filter out any momentary BLE packet collisions.
                    if consecutive_away_ticks >= 7 {
                        log::warn!(
                            "[NATIVE ENGINE] Auto-Lock triggered! RSSI: {:.1} dBm, Elapsed: {:.1}s, IsLost: {}",
                            reading.rssi, reading.seconds_since_last_packet, reading.is_lost
                        );
                        if lock_workstation().is_ok() {
                            IS_LOCKED.store(true, Ordering::SeqCst);
                            consecutive_away_ticks = 0;
                            lock_cooldown_until = now + std::time::Duration::from_secs(12);
                        }
                    }
                } else {
                    consecutive_away_ticks = 0;
                }
            }

            // EVALUATE AUTO-UNLOCK:
            if settings.auto_unlock_enabled && currently_locked {
                let is_near = !reading.is_lost
                    && reading.seconds_since_last_packet < 2.5
                    && reading.rssi >= settings.unlock_threshold_dbm as f32
                    && (!settings.require_on_wrist || (reading.is_confirmed_watch && reading.on_wrist && reading.is_unlocked));

                if is_near {
                    consecutive_near_ticks += 1;
                    // Require 3 consecutive ticks (~1.2s) of solid strong desk signal
                    if consecutive_near_ticks >= 3 {
                        log::info!(
                            "[NATIVE ENGINE] Auto-Unlock: Watch back in close desk range ({:.1} dBm). Waking display!",
                            reading.rssi
                        );
                        LockScreenController::wake_and_prompt_logon();
                        IS_LOCKED.store(false, Ordering::SeqCst);
                        session_armed = true;
                        consecutive_near_ticks = 0;
                        lock_cooldown_until = now + std::time::Duration::from_secs(12);
                    }
                } else {
                    consecutive_near_ticks = 0;
                }
            }
        }
    });
}

fn start_ipc_server() {
    let ipc = IpcServer::new();
    let _ = ipc.start(|| {
        let settings = if let Ok(s) = ENGINE_SETTINGS.lock() {
            s.clone()
        } else {
            return (false, None, false, false);
        };

        let reading = get_live_proximity(settings.tracked_mac.clone());
        let is_ready = !reading.is_lost
            && reading.seconds_since_last_packet < 2.5
            && reading.rssi >= settings.unlock_threshold_dbm as f32
            && (!settings.require_on_wrist || reading.is_confirmed_watch);

        (
            is_ready,
            Some(reading.rssi as f64),
            reading.is_confirmed_watch,
            !reading.is_confirmed_watch,
        )
    });
}

fn main() {
    env_logger::init();
    start_background_scanner();
    start_native_proximity_engine();
    start_ipc_server();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_discovered_devices,
            get_device_rssi,
            get_live_proximity,
            is_running_as_admin,
            relaunch_as_admin,
            check_for_updates,
            download_and_install_update,
            query_registry_keys,
            verify_irk,
            save_credentials,
            test_vault,
            test_workstation_lock,
            get_autostart_enabled,
            set_autostart_enabled,
            get_engine_settings,
            update_engine_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running WristKey application");
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_device_filtering_and_closest_watch_isolation() {
        let _guard = TEST_LOCK.lock().unwrap();
        if let Ok(mut lock) = DEVICES.lock() {
            lock.clear();

            // 1. User's Apple Watch SE on wrist right at desk (-54 dBm)
            lock.insert(
                "AA:BB:CC:DD:EE:01".to_string(),
                DeviceEntry {
                    device: DiscoveredDeviceDto {
                        mac: "AA:BB:CC:DD:EE:01".to_string(),
                        name: "Apple Watch (En tu muñeca y desbloqueado)".to_string(),
                        rssi: -54,
                        on_wrist: true,
                        is_unlocked: true,
                        frame_type: "Nearby Info (0x10)".to_string(),
                        is_watch_candidate: true,
                        is_confirmed_watch: true,
                        device_type: "apple_watch".to_string(),
                        last_seen_ms_ago: 0,
                    },
                    last_seen: Instant::now(),
                    smoothed_rssi_f32: -54.0,
                },
            );

            // 2. Father's Apple Watch in adjacent room (-85 dBm)
            lock.insert(
                "AA:BB:CC:DD:EE:02".to_string(),
                DeviceEntry {
                    device: DiscoveredDeviceDto {
                        mac: "AA:BB:CC:DD:EE:02".to_string(),
                        name: "Apple Watch (En tu muñeca y desbloqueado)".to_string(),
                        rssi: -85,
                        on_wrist: true,
                        is_unlocked: true,
                        frame_type: "Nearby Info (0x10)".to_string(),
                        is_watch_candidate: true,
                        is_confirmed_watch: true,
                        device_type: "apple_watch".to_string(),
                        last_seen_ms_ago: 0,
                    },
                    last_seen: Instant::now(),
                    smoothed_rssi_f32: -85.0,
                },
            );

            // 3. Noise device: non-wrist device (e.g. iPhone)
            lock.insert(
                "AA:BB:CC:DD:EE:03".to_string(),
                DeviceEntry {
                    device: DiscoveredDeviceDto {
                        mac: "AA:BB:CC:DD:EE:03".to_string(),
                        name: "iPhone (Continuity)".to_string(),
                        rssi: -45,
                        on_wrist: false,
                        is_unlocked: true,
                        frame_type: "Nearby Info (0x10)".to_string(),
                        is_watch_candidate: false,
                        is_confirmed_watch: false,
                        device_type: "apple_device".to_string(),
                        last_seen_ms_ago: 0,
                    },
                    last_seen: Instant::now(),
                    smoothed_rssi_f32: -45.0,
                },
            );
        }

        let discovered = get_discovered_devices();

        // Must ONLY return 1 device: the user's Apple Watch!
        // The non-wrist device is filtered out, and the distant father's watch is pruned.
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].mac, "AA:BB:CC:DD:EE:01");
        assert!(discovered[0].on_wrist);
        assert!(discovered[0].is_confirmed_watch);
    }

    #[test]
    fn test_live_proximity_steady_signal_and_distance() {
        let _guard = TEST_LOCK.lock().unwrap();
        let mac = "AA:BB:CC:DD:EE:99".to_string();
        if let Ok(mut lock) = DEVICES.lock() {
            lock.clear();
            lock.insert(
                mac.clone(),
                DeviceEntry {
                    device: DiscoveredDeviceDto {
                        mac: mac.clone(),
                        name: "Apple Watch (En tu muñeca)".to_string(),
                        rssi: -58,
                        on_wrist: true,
                        is_unlocked: true,
                        frame_type: "Nearby Info (0x10)".to_string(),
                        is_watch_candidate: true,
                        is_confirmed_watch: true,
                        device_type: "apple_watch".to_string(),
                        last_seen_ms_ago: 0,
                    },
                    last_seen: Instant::now(),
                    smoothed_rssi_f32: -58.0,
                },
            );
        }

        let reading = get_live_proximity(Some(mac));
        assert_eq!(reading.is_confirmed_watch, true);
        assert!((reading.rssi - (-58.0)).abs() < 0.01);
        assert!((reading.distance_meters - 1.0).abs() < 0.1);
        assert!(reading.is_near);
        assert!(!reading.is_lost);
    }

    #[test]
    fn test_mac_address_rotation_seamless_handoff() {
        let _guard = TEST_LOCK.lock().unwrap();
        let old_mac = "11:22:33:44:55:01".to_string();
        let new_mac = "11:22:33:44:55:02".to_string();

        if let Ok(mut lock) = DEVICES.lock() {
            lock.clear();

            // Old MAC has gone silent (seen 65s ago, typical after 15m RPA rotation)
            lock.insert(
                old_mac.clone(),
                DeviceEntry {
                    device: DiscoveredDeviceDto {
                        mac: old_mac.clone(),
                        name: "Apple Watch (En tu muñeca y desbloqueado)".to_string(),
                        rssi: -55,
                        on_wrist: true,
                        is_unlocked: true,
                        frame_type: "Nearby Info (0x10)".to_string(),
                        is_watch_candidate: true,
                        is_confirmed_watch: true,
                        device_type: "apple_watch".to_string(),
                        last_seen_ms_ago: 65000,
                    },
                    last_seen: Instant::now() - std::time::Duration::from_millis(65000),
                    smoothed_rssi_f32: -55.0,
                },
            );

            // New rotated MAC started advertising on wrist (seen 0.3s ago at same desk distance)
            lock.insert(
                new_mac.clone(),
                DeviceEntry {
                    device: DiscoveredDeviceDto {
                        mac: new_mac.clone(),
                        name: "Apple Watch (En tu muñeca y desbloqueado)".to_string(),
                        rssi: -54,
                        on_wrist: true,
                        is_unlocked: true,
                        frame_type: "Nearby Info (0x10)".to_string(),
                        is_watch_candidate: true,
                        is_confirmed_watch: true,
                        device_type: "apple_watch".to_string(),
                        last_seen_ms_ago: 300,
                    },
                    last_seen: Instant::now() - std::time::Duration::from_millis(300),
                    smoothed_rssi_f32: -54.0,
                },
            );
        }

        // Caller asks for old_mac, but old_mac rotated!
        // System must seamlessly hand off to new_mac without decaying or dropping signal!
        let reading = get_live_proximity(Some(old_mac));
        assert_eq!(reading.current_mac, new_mac);
        assert_eq!(reading.is_confirmed_watch, true);
        assert!(!reading.is_lost);
        assert!(reading.is_near);
        assert!((reading.rssi - (-54.0)).abs() < 0.1);
    }

    #[test]
    fn test_locked_or_off_wrist_watch_not_discovered() {
        let _guard = TEST_LOCK.lock().unwrap();
        if let Ok(mut lock) = DEVICES.lock() {
            lock.clear();

            // Watch on charger / off-wrist / locked
            lock.insert(
                "AA:BB:CC:DD:EE:04".to_string(),
                DeviceEntry {
                    device: DiscoveredDeviceDto {
                        mac: "AA:BB:CC:DD:EE:04".to_string(),
                        name: "Apple Watch (Bloqueado / Fuera de muñeca)".to_string(),
                        rssi: -50,
                        on_wrist: false,
                        is_unlocked: false,
                        frame_type: "Nearby Info (0x10)".to_string(),
                        is_watch_candidate: true,
                        is_confirmed_watch: true,
                        device_type: "apple_watch".to_string(),
                        last_seen_ms_ago: 0,
                    },
                    last_seen: Instant::now(),
                    smoothed_rssi_f32: -50.0,
                },
            );
        }

        let discovered = get_discovered_devices();
        assert_eq!(discovered.len(), 0);
    }

    #[test]
    fn test_watch_active_screen_on_or_interaction_discovered() {
        let _guard = TEST_LOCK.lock().unwrap();
        if let Ok(mut lock) = DEVICES.lock() {
            lock.clear();

            // Apple Watch SE (watchOS 10.6.2) worn on wrist, active user with screen on (0x07)
            lock.insert(
                "AA:BB:CC:DD:EE:05".to_string(),
                DeviceEntry {
                    device: DiscoveredDeviceDto {
                        mac: "AA:BB:CC:DD:EE:05".to_string(),
                        name: "Apple Watch (En tu muñeca y desbloqueado)".to_string(),
                        rssi: -52,
                        on_wrist: true,
                        is_unlocked: true,
                        frame_type: "Nearby Info (0x10)".to_string(),
                        is_watch_candidate: true,
                        is_confirmed_watch: true,
                        device_type: "apple_watch".to_string(),
                        last_seen_ms_ago: 0,
                    },
                    last_seen: Instant::now(),
                    smoothed_rssi_f32: -52.0,
                },
            );
        }

        let discovered = get_discovered_devices();
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].mac, "AA:BB:CC:DD:EE:05");
        assert!(discovered[0].on_wrist);
        assert!(discovered[0].is_confirmed_watch);
    }

    #[test]
    fn test_iphone_active_screen_on_not_discovered() {
        let _guard = TEST_LOCK.lock().unwrap();
        if let Ok(mut lock) = DEVICES.lock() {
            lock.clear();

            // iPhone active with screen on on desk: not on wrist, rejected
            lock.insert(
                "AA:BB:CC:DD:EE:06".to_string(),
                DeviceEntry {
                    device: DiscoveredDeviceDto {
                        mac: "AA:BB:CC:DD:EE:06".to_string(),
                        name: "iPhone (Activo / Pantalla encendida)".to_string(),
                        rssi: -42,
                        on_wrist: false,
                        is_unlocked: true,
                        frame_type: "Nearby Info (0x10)".to_string(),
                        is_watch_candidate: false,
                        is_confirmed_watch: false,
                        device_type: "apple_device".to_string(),
                        last_seen_ms_ago: 0,
                    },
                    last_seen: Instant::now(),
                    smoothed_rssi_f32: -42.0,
                },
            );
        }

        let discovered = get_discovered_devices();
        assert_eq!(discovered.len(), 0);
    }

    #[test]
    fn test_desk_watch_and_iphone_watch_wins_isolation() {
        let _guard = TEST_LOCK.lock().unwrap();
        if let Ok(mut lock) = DEVICES.lock() {
            lock.clear();

            // User's Apple Watch SE on wrist at desk (-52 dBm)
            lock.insert(
                "AA:BB:CC:DD:EE:07".to_string(),
                DeviceEntry {
                    device: DiscoveredDeviceDto {
                        mac: "AA:BB:CC:DD:EE:07".to_string(),
                        name: "Apple Watch (En tu muñeca y desbloqueado)".to_string(),
                        rssi: -52,
                        on_wrist: true,
                        is_unlocked: true,
                        frame_type: "Nearby Info (0x10)".to_string(),
                        is_watch_candidate: true,
                        is_confirmed_watch: true,
                        device_type: "apple_watch".to_string(),
                        last_seen_ms_ago: 0,
                    },
                    last_seen: Instant::now(),
                    smoothed_rssi_f32: -52.0,
                },
            );

            // User's iPhone on desk with screen on (-44 dBm, slightly stronger RF)
            lock.insert(
                "AA:BB:CC:DD:EE:08".to_string(),
                DeviceEntry {
                    device: DiscoveredDeviceDto {
                        mac: "AA:BB:CC:DD:EE:08".to_string(),
                        name: "Apple Watch (Detectado en escritorio)".to_string(),
                        rssi: -44,
                        on_wrist: true,
                        is_unlocked: true,
                        frame_type: "Nearby Info (0x10)".to_string(),
                        is_watch_candidate: true,
                        is_confirmed_watch: false,
                        device_type: "apple_candidate".to_string(),
                        last_seen_ms_ago: 0,
                    },
                    last_seen: Instant::now(),
                    smoothed_rssi_f32: -44.0,
                },
            );
        }

        // Even though iPhone has slightly higher raw RSSI, Confirmed Apple Watch on wrist MUST WIN!
        let discovered = get_discovered_devices();
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].mac, "AA:BB:CC:DD:EE:07");
        assert!(discovered[0].is_confirmed_watch);
    }

    #[test]
    fn test_distant_watch_in_adjacent_room_not_hijacked_on_user_departure() {
        let _guard = TEST_LOCK.lock().unwrap();
        let user_mac = "AA:BB:CC:DD:EE:10".to_string();
        let distant_mac = "6A:81:D5:37:B0:8B".to_string();

        if let Ok(mut lock) = DEVICES.lock() {
            lock.clear();

            // User walked away: user's watch went silent (seen 10s ago)
            lock.insert(
                user_mac.clone(),
                DeviceEntry {
                    device: DiscoveredDeviceDto {
                        mac: user_mac.clone(),
                        name: "Apple Watch (En tu muñeca y desbloqueado)".to_string(),
                        rssi: -58,
                        on_wrist: true,
                        is_unlocked: true,
                        frame_type: "Nearby Info (0x10)".to_string(),
                        is_watch_candidate: true,
                        is_confirmed_watch: true,
                        device_type: "apple_watch".to_string(),
                        last_seen_ms_ago: 10000,
                    },
                    last_seen: Instant::now() - std::time::Duration::from_millis(10000),
                    smoothed_rssi_f32: -58.0,
                },
            );

            // Distant Apple Watch in adjacent room (father's watch, -81 dBm) is advertising continuously (seen 200ms ago)
            lock.insert(
                distant_mac.clone(),
                DeviceEntry {
                    device: DiscoveredDeviceDto {
                        mac: distant_mac.clone(),
                        name: "Apple Watch (En tu muñeca y desbloqueado)".to_string(),
                        rssi: -81,
                        on_wrist: true,
                        is_unlocked: true,
                        frame_type: "Nearby Info (0x10)".to_string(),
                        is_watch_candidate: true,
                        is_confirmed_watch: true,
                        device_type: "apple_watch".to_string(),
                        last_seen_ms_ago: 200,
                    },
                    last_seen: Instant::now() - std::time::Duration::from_millis(200),
                    smoothed_rssi_f32: -81.0,
                },
            );
        }

        // Caller tracks user_mac. The user walked away.
        // It MUST NOT hijack distant_mac! It must stay on user_mac and decay signal!
        let reading = get_live_proximity(Some(user_mac.clone()));
        assert_eq!(reading.current_mac, user_mac);
        assert_ne!(reading.current_mac, distant_mac);
        // Signal must have decayed because elapsed > 2.0s
        assert!(reading.rssi < -58.0);
    }

    #[test]
    fn test_iphone_with_screen_on_strictly_rejected_as_apple_watch() {
        use wristkey_core::continuity::AppleContinuityFrame;
        // Raw Nearby Info packet from an active iPhone on the desk:
        // Type 0x10, Len 0x05, Action 0x17 (Screen On with iCloud), Status 0x04 (Wi-Fi ON, Unlocked)
        let raw_iphone = vec![0x10, 0x05, 0x17, 0x04, 0xAA, 0xBB, 0xCC];
        let frame = AppleContinuityFrame::parse(&raw_iphone).expect("Must parse");

        // An iPhone must NEVER be confirmed as an Apple Watch and must NEVER be on wrist!
        assert!(!frame.is_confirmed_apple_watch());
        assert!(!frame.is_on_wrist);
        assert!(!frame.is_on_wrist_and_unlocked);
        assert!(!frame.is_apple_watch());
    }

    #[test]
    fn test_apple_watch_se_hardware_device_class_discovered_and_on_wrist() {
        let _guard = TEST_LOCK.lock().unwrap();
        use wristkey_core::continuity::AppleContinuityFrame;
        // User's Apple Watch SE (watchOS 10.6.2) live capture packet:
        let raw_watch = vec![0x10, 0x07, 0x3B, 0x1F, 0x2E, 0xD6, 0xDB, 0x68, 0x28];
        let frame = AppleContinuityFrame::parse(&raw_watch).expect("Must parse Watch SE frame");

        assert!(frame.is_confirmed_apple_watch());
        assert!(frame.is_on_wrist);
        assert!(frame.is_on_wrist_and_unlocked);

        let mac = "68:DF:DA:84:95:A9".to_string();
        if let Ok(mut lock) = DEVICES.lock() {
            lock.clear();
            lock.insert(
                mac.clone(),
                DeviceEntry {
                    device: DiscoveredDeviceDto {
                        mac: mac.clone(),
                        name: "Apple Watch (En tu muñeca y desbloqueado)".to_string(),
                        rssi: -45,
                        on_wrist: frame.is_on_wrist_and_unlocked,
                        is_unlocked: !frame.is_locked,
                        frame_type: "Nearby Info (0x10)".to_string(),
                        is_watch_candidate: true,
                        is_confirmed_watch: frame.is_confirmed_apple_watch(),
                        device_type: "apple_watch".to_string(),
                        last_seen_ms_ago: 0,
                    },
                    last_seen: Instant::now(),
                    smoothed_rssi_f32: -45.0,
                },
            );
        }

        let discovered = get_discovered_devices();
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].mac, mac);
        assert!(discovered[0].is_confirmed_watch);
        assert!(discovered[0].on_wrist);
        assert_eq!(discovered[0].rssi, -45);
    }

    #[test]
    fn test_watch_on_charger_and_silence_reported_off_wrist() {
        let _guard = TEST_LOCK.lock().unwrap();
        let charger_mac = "73:DD:A6:75:F4:B2".to_string();

        if let Ok(mut lock) = DEVICES.lock() {
            lock.clear();
            // Watch on charger: Action 0x05 -> on_wrist = false
            lock.insert(
                charger_mac.clone(),
                DeviceEntry {
                    device: DiscoveredDeviceDto {
                        mac: charger_mac.clone(),
                        name: "Apple Watch (Bloqueado / Fuera de muñeca / En cargador)".to_string(),
                        rssi: -44,
                        on_wrist: false,
                        is_unlocked: false,
                        frame_type: "Nearby Info (0x10)".to_string(),
                        is_watch_candidate: true,
                        is_confirmed_watch: true,
                        device_type: "apple_watch".to_string(),
                        last_seen_ms_ago: 100,
                    },
                    last_seen: Instant::now() - std::time::Duration::from_millis(100),
                    smoothed_rssi_f32: -44.0,
                },
            );
        }

        // Live proximity reading MUST report on_wrist = false and off-wrist name!
        let reading = get_live_proximity(Some(charger_mac.clone()));
        assert!(!reading.on_wrist);
        assert!(!reading.is_unlocked);
        assert!(reading.device_name.contains("cargador") || reading.device_name.contains("Cargador"));

        // When a watch goes silent for > 3.5 seconds:
        if let Ok(mut lock) = DEVICES.lock() {
            if let Some(entry) = lock.get_mut(&charger_mac) {
                entry.device.on_wrist = true; // Even if it was on wrist before!
                entry.last_seen = Instant::now() - std::time::Duration::from_millis(4000);
            }
        }

        let reading_silent = get_live_proximity(Some(charger_mac));
        assert!(!reading_silent.on_wrist);
        assert!(!reading_silent.is_unlocked);
        assert!(reading_silent.device_name.contains("Sin señal reciente") || reading_silent.device_name.contains("Fuera de muñeca"));
    }
}


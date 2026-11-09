#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use wristkey_core::crypto::IrkResolver;
use wristkey_core::dpapi::{CredentialVault, UserCredentials};
use wristkey_core::lock::lock_workstation;
use wristkey_core::registry::{read_paired_bluetooth_keys, PairedBluetoothKey};
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
}

static DEVICES: std::sync::LazyLock<Arc<Mutex<HashMap<String, DeviceEntry>>>> =
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

#[tauri::command]
fn get_discovered_devices() -> Vec<DiscoveredDeviceDto> {
    let now = Instant::now();
    let mut results = Vec::new();

    if let Ok(mut lock) = DEVICES.lock() {
        // Drop devices not seen in the last 12 seconds (ample window for idle watch BLE intervals)
        lock.retain(|_, entry| now.duration_since(entry.last_seen).as_secs() < 12);

        for entry in lock.values() {
            let mut dev = entry.device.clone();
            dev.last_seen_ms_ago = now.duration_since(entry.last_seen).as_millis() as u64;
            results.push(dev);
        }
    }

    // Sort order:
    // 1. Confirmed Apple Watches FIRST (sorted by RSSI descending)
    // 2. Other Apple devices SECOND (sorted by RSSI descending)
    results.sort_by(|a, b| {
        b.is_confirmed_watch
            .cmp(&a.is_confirmed_watch)
            .then_with(|| b.rssi.cmp(&a.rssi))
    });
    results
}

#[tauri::command]
fn get_live_proximity(mac: Option<String>) -> ProximityReadingDto {
    let now = Instant::now();
    if let Ok(lock) = DEVICES.lock() {
        // Find the target to monitor:
        // If a specific MAC is given, monitor THAT device without hopping around!
        let target = if let Some(ref m) = mac {
            lock.get(m).or_else(|| {
                // Only if the specific MAC does not exist at all, pick the closest confirmed watch
                let mut watches: Vec<_> = lock.values().filter(|e| e.device.is_confirmed_watch).collect();
                watches.sort_by(|a, b| b.device.rssi.cmp(&a.device.rssi));
                watches.first().copied()
            })
        } else {
            let mut watches: Vec<_> = lock.values().filter(|e| e.device.is_confirmed_watch).collect();
            watches.sort_by(|a, b| b.device.rssi.cmp(&a.device.rssi));
            watches.first().copied().or_else(|| {
                let mut all: Vec<_> = lock.values().collect();
                all.sort_by(|a, b| b.device.rssi.cmp(&a.device.rssi));
                all.first().copied()
            })
        };

        if let Some(entry) = target {
            let elapsed_secs = now.duration_since(entry.last_seen).as_secs_f32();

            // NATURAL SIGNAL RETENTION & SMOOTH OUT-OF-RANGE TRANSITION:
            // 0s to 4.5s: Normal BLE advertisement interval. Signal is STABLE at measured RSSI.
            // 4.5s to 9.0s: User has walked out of range / packets dropped. Signal gently fades towards -88 dBm.
            // > 9.0s: Completely lost / disconnected.
            let effective_rssi = if elapsed_secs > 4.5 {
                let fade = (elapsed_secs - 4.5) * 5.0; // Fades at 5 dBm per second after 4.5s
                (entry.device.rssi as f32 - fade).max(-88.0)
            } else {
                entry.device.rssi as f32
            };

            // Distance estimation (path loss model with desk anchor)
            let ratio = (-48.0 - effective_rssi) / (10.0 * 2.2);
            let distance_meters = (10.0_f32).powf(ratio).clamp(0.2, 8.0);

            let is_lost = elapsed_secs > 7.0 || effective_rssi <= -80.0;
            let is_near = effective_rssi >= -58.0 && !is_lost;

            let status_text = if is_lost {
                "LEJOS / SIN SEÑAL (Zona de bloqueo)".to_string()
            } else if is_near {
                "CERCA (En tu escritorio / Desbloqueado)".to_string()
            } else if effective_rssi <= -72.0 {
                "ALEJÁNDOTE (Próximo al bloqueo)".to_string()
            } else {
                "ZONA MEDIA (En la habitación)".to_string()
            };

            return ProximityReadingDto {
                current_mac: entry.device.mac.clone(),
                device_name: entry.device.name.clone(),
                is_confirmed_watch: entry.device.is_confirmed_watch,
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
        device_name: "Buscando reloj...".to_string(),
        is_confirmed_watch: false,
        rssi: -88.0,
        smoothed_rssi: -88.0,
        distance_meters: 8.0,
        is_near: false,
        is_lost: true,
        seconds_since_last_packet: 10.0,
        status_text: "SIN SEÑAL (Coloca tu reloj cerca)".to_string(),
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

    let status = std::process::Command::new("powershell.exe")
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
fn check_for_updates() -> Result<Option<UpdateInfo>, String> {
    let current_version = env!("CARGO_PKG_VERSION");

    let output = std::process::Command::new("curl.exe")
        .args(&[
            "-s",
            "-H",
            "User-Agent: WristKey",
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
}

#[tauri::command]
fn download_and_install_update(download_url: String) -> Result<(), String> {
    std::thread::spawn(move || {
        let temp_dir = std::env::temp_dir();
        let target_exe = temp_dir.join("WristKey-Update.exe");
        let target_path = target_exe.to_string_lossy().to_string();

        let status = std::process::Command::new("curl.exe")
            .args(&["-L", "-s", "-o", &target_path, &download_url])
            .status();

        if let Ok(s) = status {
            if s.success() {
                let _ = std::process::Command::new(&target_exe).spawn();
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

fn start_background_scanner() {
    std::thread::spawn(|| {
        let mut scanner = BleScanner::new();
        match scanner.start() {
            Ok(rx) => {
                log::info!("Live WinRT BLE scanner started for UI");
                while let Ok(event) = rx.recv() {
                    if let Some(frame) = event.apple_frame {
                        // STRICT FILTER: Only accept Apple Watch candidates (Nearby Info 0x10)
                        if frame.message_type != 0x10 {
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

                        if let Ok(mut lock) = DEVICES.lock() {
                            let was_already_watch = lock.get(&mac_str).map_or(false, |e| e.device.is_confirmed_watch);
                            let is_confirmed_watch = was_already_watch || frame.is_apple_watch();
                            let device_type = if is_confirmed_watch {
                                "apple_watch".to_string()
                            } else {
                                "apple_device".to_string()
                            };

                            let name = if frame.is_on_wrist_and_unlocked {
                                "Apple Watch (En muñeca y desbloqueado)".to_string()
                            } else if frame.is_on_wrist {
                                "Apple Watch (En muñeca, bloqueado)".to_string()
                            } else if is_confirmed_watch {
                                "Apple Watch".to_string()
                            } else {
                                "iPhone / iPad (Continuity)".to_string()
                            };

                            let smoothed_rssi = if let Some(existing) = lock.get(&mac_str) {
                                let diff = (event.rssi - existing.device.rssi).abs();
                                // Responsive dynamic smoothing: if moving (diff >= 3 dBm), take 70% of new reading immediately
                                let alpha = if diff >= 3 { 0.70 } else { 0.40 };
                                ((existing.device.rssi as f32 * (1.0 - alpha)) + (event.rssi as f32 * alpha)).round() as i16
                            } else {
                                event.rssi
                            };

                            let dto = DiscoveredDeviceDto {
                                mac: mac_str.clone(),
                                name,
                                rssi: smoothed_rssi,
                                on_wrist: frame.is_on_wrist || is_confirmed_watch,
                                is_unlocked: !frame.is_locked,
                                frame_type: "Nearby Info (0x10)".to_string(),
                                is_watch_candidate: is_confirmed_watch,
                                is_confirmed_watch,
                                device_type,
                                last_seen_ms_ago: 0,
                            };

                            lock.insert(
                                mac_str,
                                DeviceEntry {
                                    device: dto,
                                    last_seen: Instant::now(),
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

fn main() {
    env_logger::init();
    start_background_scanner();

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
        ])
        .run(tauri::generate_context!())
        .expect("error while running WristKey application");
}

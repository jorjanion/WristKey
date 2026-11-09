#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

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
    smoothed_rssi_f32: f32,
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
            // ONLY return confirmed Apple Watches detected on wrist
            if entry.device.is_confirmed_watch && entry.device.on_wrist {
                let mut dev = entry.device.clone();
                dev.last_seen_ms_ago = now.duration_since(entry.last_seen).as_millis() as u64;
                results.push(dev);
            }
        }
    }

    // Sort order: Closest Apple Watch first (highest RSSI)
    results.sort_by(|a, b| b.rssi.cmp(&a.rssi));

    // The user explicitly requires that ONLY their Apple Watch appear in the scan:
    // If multiple watches exist in the house (e.g. user's watch at desk, father's watch in adjacent room),
    // strictly isolate the user's Apple Watch (the closest one at the desk).
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
                if elapsed >= 3.0 {
                    // Check if the watch rotated its MAC address:
                    // Look for another confirmed on-wrist watch seen much more recently (< 2.5s ago)
                    let fresh_watch = lock
                        .values()
                        .filter(|e| {
                            e.device.mac != *m
                                && e.device.is_confirmed_watch
                                && e.device.on_wrist
                                && now.duration_since(e.last_seen).as_secs_f32() < 2.5
                        })
                        .max_by(|a, b| a.device.rssi.cmp(&b.device.rssi));

                    Some(fresh_watch.unwrap_or(entry))
                } else {
                    Some(entry)
                }
            } else {
                let mut watches: Vec<_> = lock
                    .values()
                    .filter(|e| e.device.is_confirmed_watch && e.device.on_wrist)
                    .collect();
                watches.sort_by(|a, b| b.device.rssi.cmp(&a.device.rssi));
                watches.first().copied()
            }
        } else {
            let mut watches: Vec<_> = lock
                .values()
                .filter(|e| e.device.is_confirmed_watch && e.device.on_wrist)
                .collect();
            watches.sort_by(|a, b| b.device.rssi.cmp(&a.device.rssi));
            watches.first().copied()
        };

        if let Some(entry) = target {
            let elapsed_secs = now.duration_since(entry.last_seen).as_secs_f32();

            // NATURAL SIGNAL RETENTION & SMOOTH OUT-OF-RANGE TRANSITION:
            // 0s to 6.0s: Normal BLE advertisement interval on wrist (covers idle screen sleep & dropped packet leeway).
            //             Signal remains 100% ROCK SOLID at smoothed value without yo-yo drops.
            // 6.0s to 10.0s: User walking out of range / packets stopped. Gentle smooth linear decay.
            // > 10.0s: Completely lost / disconnected.
            let effective_rssi = if elapsed_secs > 6.0 {
                let fade = (elapsed_secs - 6.0) * 3.0;
                (entry.smoothed_rssi_f32 - fade).max(-88.0)
            } else {
                entry.smoothed_rssi_f32
            };

            // Accurate Log-Distance Path Loss Model:
            // Reference at 1m is -58.0 dBm for Apple Watch on wrist; path loss exponent n = 2.4.
            let ratio = (-58.0 - effective_rssi) / (10.0 * 2.4);
            let distance_meters = (10.0_f32).powf(ratio).clamp(0.3, 8.0);

            let is_lost = elapsed_secs > 10.0 || effective_rssi <= -80.0;
            let is_near = effective_rssi >= -60.0 && !is_lost;

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
        device_name: "Buscando tu Apple Watch...".to_string(),
        is_confirmed_watch: false,
        rssi: -88.0,
        smoothed_rssi: -88.0,
        distance_meters: 8.0,
        is_near: false,
        is_lost: true,
        seconds_since_last_packet: 10.0,
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

fn start_background_scanner() {
    std::thread::spawn(|| {
        let mut scanner = BleScanner::new();
        match scanner.start() {
            Ok(rx) => {
                log::info!("Live WinRT BLE scanner started for UI");
                while let Ok(event) = rx.recv() {
                    if let Some(frame) = event.apple_frame {
                        // Only Nearby Info frames (0x10) carry Apple Watch on-wrist telemetry
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

                        // Strictly require Apple Watch on-wrist frame (0x0A)
                        let is_on_wrist_frame = frame.action_code == Some(wristkey_core::continuity::ACTION_WATCH_ON_WRIST_UNLOCKED)
                            && !frame.is_locked;

                        if let Ok(mut lock) = DEVICES.lock() {
                            let was_already_watch = lock.get(&mac_str).map_or(false, |e| e.device.is_confirmed_watch);

                            // STRICT NOISE FILTER:
                            // Drop everything that is not an on-wrist Apple Watch.
                            // Filters out iPhones, iPads, MacBooks, AirPods, and non-wrist devices completely.
                            if !is_on_wrist_frame && !was_already_watch {
                                continue;
                            }

                            // If watch was locked or taken off wrist, mark it off-wrist
                            let is_on_wrist = is_on_wrist_frame && !frame.is_locked;
                            let is_confirmed_watch = true;

                            let name = if is_on_wrist {
                                "Apple Watch (En tu muñeca y desbloqueado)".to_string()
                            } else {
                                "Apple Watch (Bloqueado / Fuera de muñeca)".to_string()
                            };

                            // HIGH-PRECISION ADAPTIVE RSSI SMOOTHING:
                            let raw_f = event.rssi as f32;
                            let smoothed_rssi_f32 = if let Some(existing) = lock.get(&mac_str) {
                                let prev = existing.smoothed_rssi_f32;
                                let diff = (raw_f - prev).abs();

                                // Outlier clamp for physical sanity (> 20 dB spikes due to RF collision)
                                let clamped_raw = if (raw_f - prev) > 20.0 {
                                    prev + 20.0
                                } else if (prev - raw_f) > 20.0 {
                                    prev - 20.0
                                } else {
                                    raw_f
                                };

                                // Small RF noise (< 5 dBm): Alpha = 0.18 -> rock-solid steady reading
                                // Medium movement (5-10 dBm): Alpha = 0.35 -> smooth transition
                                // Large movement (>= 10 dBm): Alpha = 0.55 -> rapid tracking
                                let alpha = if diff < 5.0 {
                                    0.18
                                } else if diff < 10.0 {
                                    0.35
                                } else {
                                    0.55
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
                                frame_type: "Nearby Info (0x10)".to_string(),
                                is_watch_candidate: true,
                                is_confirmed_watch,
                                device_type: "apple_watch".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_filtering_and_closest_watch_isolation() {
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
        let old_mac = "11:22:33:44:55:01".to_string();
        let new_mac = "11:22:33:44:55:02".to_string();

        if let Ok(mut lock) = DEVICES.lock() {
            lock.clear();

            // Old MAC has gone silent (seen 4.0s ago)
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
                        last_seen_ms_ago: 4000,
                    },
                    last_seen: Instant::now() - std::time::Duration::from_millis(4000),
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
}


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
    pub last_seen_ms_ago: u64,
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
        // Retain only devices seen within the last 15 seconds
        lock.retain(|_, entry| now.duration_since(entry.last_seen).as_secs() < 15);

        for entry in lock.values() {
            let mut dev = entry.device.clone();
            dev.last_seen_ms_ago = now.duration_since(entry.last_seen).as_millis() as u64;
            results.push(dev);
        }
    }

    // Sort by RSSI descending (strongest / nearest device first)
    results.sort_by(|a, b| b.rssi.cmp(&a.rssi));
    results
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
                        let mac_str = format!(
                            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                            (event.mac >> 40) & 0xFF,
                            (event.mac >> 32) & 0xFF,
                            (event.mac >> 24) & 0xFF,
                            (event.mac >> 16) & 0xFF,
                            (event.mac >> 8) & 0xFF,
                            event.mac & 0xFF
                        );

                        let is_watch_candidate = frame.message_type == 0x10 || frame.is_on_wrist;

                        let name = if frame.is_on_wrist_and_unlocked {
                            "Apple Watch (En muñeca y desbloqueado)".to_string()
                        } else if frame.is_on_wrist {
                            "Apple Watch (En muñeca, bloqueado)".to_string()
                        } else if frame.message_type == 0x10 {
                            "Apple Watch / iPhone (Nearby Info 0x10)".to_string()
                        } else if frame.message_type == 0x12 {
                            "Dispositivo Apple (Find My / AirTag)".to_string()
                        } else {
                            format!("Dispositivo Apple ({})", frame.message_type_name)
                        };

                        let dto = DiscoveredDeviceDto {
                            mac: mac_str.clone(),
                            name,
                            rssi: event.rssi,
                            on_wrist: frame.is_on_wrist,
                            is_unlocked: !frame.is_locked,
                            frame_type: frame.message_type_name,
                            is_watch_candidate,
                            last_seen_ms_ago: 0,
                        };

                        if let Ok(mut lock) = DEVICES.lock() {
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

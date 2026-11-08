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
    pub last_seen_ms_ago: u64,
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
        // Retain only devices seen within the last 20 seconds
        lock.retain(|_, entry| now.duration_since(entry.last_seen).as_secs() < 20);

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

                        let name = if frame.is_on_wrist_and_unlocked {
                            "Apple Watch (On Wrist & Unlocked)".to_string()
                        } else if frame.is_on_wrist {
                            "Apple Watch (On Wrist, Locked)".to_string()
                        } else if frame.message_type == 0x10 {
                            "Apple Watch / iPhone (Nearby Info 0x10)".to_string()
                        } else {
                            format!("Apple Device ({})", frame.message_type_name)
                        };

                        let dto = DiscoveredDeviceDto {
                            mac: mac_str.clone(),
                            name,
                            rssi: event.rssi,
                            on_wrist: frame.is_on_wrist,
                            is_unlocked: !frame.is_locked,
                            frame_type: frame.message_type_name,
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
            query_registry_keys,
            verify_irk,
            save_credentials,
            test_vault,
            test_workstation_lock,
        ])
        .run(tauri::generate_context!())
        .expect("error while running WristKey application");
}

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use wristkey_core::dpapi::{CredentialVault, UserCredentials};
use wristkey_core::lock::lock_workstation;
use wristkey_core::registry::{read_paired_bluetooth_keys, PairedBluetoothKey};
use wristkey_core::crypto::IrkResolver;

#[tauri::command]
fn query_registry_keys() -> Result<Vec<PairedBluetoothKey>, String> {
    read_paired_bluetooth_keys().map_err(|e| e.to_string())
}

#[tauri::command]
fn verify_irk(irk: String) -> Result<bool, String> {
    IrkResolver::from_hex(&irk).map(|_| true).map_err(|e| e.to_string())
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

fn main() {
    env_logger::init();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            query_registry_keys,
            verify_irk,
            save_credentials,
            test_vault,
            test_workstation_lock,
        ])
        .run(tauri::generate_context!())
        .expect("error while running WristKey application");
}

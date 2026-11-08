# WristKey ⌚🔑

[![Windows 11 / 10](https://img.shields.io/badge/Windows-11%20%7C%2010-0078D4?logo=windows&logoColor=white)](https://github.com/jorjanion/WristKey)
[![License: MIT / Apache 2.0](https://img.shields.io/badge/License-MIT%20%2F%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![Release](https://img.shields.io/github/v/release/jorjanion/WristKey?color=2ea44f&label=Release)](https://github.com/jorjanion/WristKey/releases)

**WristKey** brings Apple-style proximity auto-lock and seamless unlock to Windows 11 and 10 using your **Apple Watch** over Bluetooth Low Energy (BLE).

Similar to native macOS Auto Unlock and Linux `watch-unlock-rs`, WristKey continuously senses your Apple Watch in the background:
- 🚶 **Step away from your desk?** Windows locks your workstation automatically.
- 🪑 **Sit back down with your watch?** Windows detects proximity, wakes the screen, and unlocks your session instantly.
- ⌚ **Take off your watch?** The watch locks itself, and WristKey immediately denies access until you put it back on and enter your passcode.

Designed for **everyone** (both technical and non-technical users): comes with a single-click installer (`.exe` / `.msi`) and a clean Windows 11 Fluent Design setup wizard. **No terminal or code editing required.**

---

## 🚀 Quick Download & Setup (No Terminal Needed)

### 1. Download the Installer
Go to the **[Latest GitHub Releases](https://github.com/jorjanion/WristKey/releases)** and download:
* **`WristKey_0.4.0_x64-setup.exe`** (Standard Windows Installer) or
* **`WristKey_0.4.0_x64.msi`** (Enterprise / Windows Installer Package)

Double-click to install. It installs WristKey to your Start Menu and Desktop.

### 2. Follow the 5-Step Setup Wizard
Launch **WristKey** from your desktop or Start Menu. The Fluent 2 Wizard walks you through:
1. **Radar Discovery:** Real-time animated radar detects nearby Apple Watch BLE beacons.
2. **Apple IRK Pairing:** Automatically imports your watch's Identity Resolving Key from Windows Bluetooth settings (`BTHPORT`) with one click.
3. **Live Distance Calibrator:** Interactive sliders allow you to customize your Walk-Away Lock distance (e.g., 3 meters / -82 dBm) and Approach Unlock distance (e.g., 1 meter / -62 dBm).
4. **Secure Credential Vault:** Protects your Windows login PIN or password using native Windows DPAPI (`CryptProtectData`), tied strictly to your Windows user account master key.
5. **Dashboard & Tray:** Minimize to system tray. WristKey runs silently in the background with near-zero CPU and battery usage.

---

## 🛡️ Security & Privacy Architecture

WristKey is built with strict privacy and enterprise-grade security standards in mind:

1. **Resolves Apple's Rotating MAC (RPA):**
   Apple devices rotate their Bluetooth MAC address every ~15 minutes to prevent tracking. WristKey resolves Resolvable Private Addresses in sub-microsecond time using your paired watch's Identity Resolving Key (**IRK**) with hardware AES-128 (`ah(IRK, prand)`).
2. **Wrist Detection & Lock State Filtering:**
   Parses Apple Continuity frames (`0x004C`, type `0x10` Nearby Info). If you remove your watch or lock it with your passcode, watchOS flags the change in under a second; WristKey rejects proximity until re-authenticated.
3. **Hardware-Tied Windows DPAPI Encryption:**
   Your Windows unlock credentials are encrypted with Windows DPAPI (`CryptProtectData`). Plaintext passwords are never saved on disk, never sent over the network, and only decrypted in-memory during an authorized unlock sequence.
4. **Isolated Named Pipe IPC:**
   Authentication signals between the background daemon and the Windows Credential Provider communicate over a restricted local Named Pipe (`\\.\pipe\WristKeyUnlockPipe`).

```
Apple Watch (BLE Advertisement)
  │  (RPA rotated every 15 min + Apple Continuity telemetry)
  ▼
Windows BLE Watcher (WinRT)
  ▼
RPA Resolver (AES-128 IRK matching via Bluetooth Core Spec)
  ▼
Apple Continuity Parser (0x004C / 0x10 Nearby Info)
  │  Ensures watch is ON WRIST and UNLOCKED (Action Code 0x0A)
  ▼
RSSI EMA Filter & Hysteresis State Machine
  ├── Walk Away (RSSI < -82 dBm)  ──> LockWorkStation() [Windows Lock Screen]
  └── Approach (RSSI > -62 dBm)   ──> Wake Screen & Unlock via Credential Provider
```

---

## 🛠️ For Developers & Power Users (CLI & Source)

If you are a developer and want to inspect raw Apple Continuity packets, test the cryptographic engine, or run WristKey from the terminal:

### Prerequisites
* Windows 11 or Windows 10 (version 1903+)
* Bluetooth 4.2+ adapter with LE support
* [Rust](https://rustup.rs/) (edition 2021+)

### Building from Source

```powershell
# Clone the repository
git clone https://github.com/jorjanion/WristKey.git
cd WristKey

# Run unit and cryptographic tests
cargo test

# Launch the Windows 11 Fluent UI from source
cargo run -p wristkey-ui
```

### Advanced CLI Commands

The CLI crate (`crates/wristkey-cli`) provides specialized diagnostic tools:

```powershell
# 1. Verify RPA AES-128 engine against Bluetooth SIG official test vectors
cargo run -p wristkey-cli -- --verify-crypto

# 2. Live BLE packet scanner with Apple Continuity frame decoding
cargo run -p wristkey-cli -- --scan

# 3. Terminal distance & signal meter calibration
cargo run -p wristkey-cli -- --calibrate

# 4. Dump paired Bluetooth LE Identity Resolving Keys (IRK) from Windows Registry
cargo run -p wristkey-cli -- --list-keys

# 5. Continuous Apple Continuity telemetry (Action Code, on-wrist, passcode status)
cargo run -p wristkey-cli -- --telemetry

# 6. Configure & test DPAPI credential vault
cargo run -p wristkey-cli -- --setup-credentials
cargo run -p wristkey-cli -- --test-vault

# 7. Run headless background auto-lock and unlock
cargo run -p wristkey-cli -- --auto-lock --auto-unlock --lock-threshold -82 --unlock-threshold -62
```

---

## 📂 Project Structure

```
WristKey/
├── .github/workflows/       # GitHub Actions automated release pipeline (.exe & .msi)
├── crates/
│   ├── wristkey-core/       # BLE scanner, RPA crypto, Apple Continuity, DPAPI, RSSI filter
│   ├── wristkey-cli/        # Terminal diagnostics, packet inspector, registry dumper
│   ├── wristkey-credprov/   # Windows Credential Provider COM DLL for desktop unlock
│   ├── wristkey-daemon/     # Headless background service and tray worker
│   └── wristkey-ui/         # Windows 11 Fluent UI Setup Wizard (Tauri v2 + WebView2)
└── README.md
```

---

## 🗺️ Roadmap & Status

- [x] **Phase 1: Core BLE & Proximity Auto-Lock** (WinRT watcher, Apple Continuity 0x004C parser, RPA resolver, RSSI EMA smoothing).
- [x] **Phase 2: IRK Registry Extraction & On-Wrist Detection** (`HKLM\...\BTHPORT` keys, Action Code 0x0A bitmask, preventive lock).
- [x] **Phase 3: Secure Unlock Mechanism** (DPAPI credential vault, Named Pipe IPC, Windows Credential Provider DLL, SAS wake controller).
- [x] **Phase 4: Windows 11 Fluent UI & Automated Installers** (Tauri v2 Setup Wizard, visual radar, distance slider, NSIS `.exe` & WiX `.msi` packaging).

---

## 📜 License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE).

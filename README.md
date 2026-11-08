# WristKey ⌚🔑

**WristKey** brings Apple-style proximity auto-lock and unlock to Windows 11 and 10 using your **Apple Watch** over Bluetooth Low Energy (BLE).

Similar to native macOS Auto Unlock and Linux projects like `watch-unlock-rs`, WristKey continuously tracks your Apple Watch in the background and locks your workstation when you step away.

---

## How It Works

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
  ├── RSSI < -82 dBm  ──> LockWorkStation() [Windows Lock Screen]
  └── RSSI > -62 dBm  ──> Signal in Proximity (Unlock candidate)
```

1. **Resolves Apple's Rotating MAC (RPA):** Apple devices rotate their Bluetooth MAC address every ~15 minutes to prevent tracking. WristKey resolves Resolvable Private Addresses in sub-microsecond time using your paired watch's Identity Resolving Key (**IRK**) with hardware AES-128 (`ah(IRK, prand)`).
2. **On-Wrist & Unlocked Filter:** Listens to Apple Continuity telemetry (`0x004C`, type `0x10` Nearby Info). If you take the watch off your wrist or it locks, WristKey immediately stops treating it as an authorized presence token.
3. **Flicker-Free Proximity:** Uses an Exponential Moving Average (EMA) and a dual-threshold hysteresis model to avoid false triggers when moving your hands.

---

## Project Structure

This repository is organized as a Cargo workspace:

* [`crates/wristkey-core`](crates/wristkey-core): Core library containing WinRT BLE scanning, Apple Continuity parser, IRK cryptographic resolver, RSSI smoothing filter, and Windows lock primitives.
* [`crates/wristkey-cli`](crates/wristkey-cli): Interactive CLI for real-time packet inspection, distance calibration, and auto-lock testing.
* [`crates/wristkey-daemon`](crates/wristkey-daemon) *(Phase 2/3)*: Low-footprint Windows background service and system tray agent.
* [`crates/wristkey-credprov`](crates/wristkey-credprov) *(Phase 3)*: Windows Credential Provider V2 for seamless desktop unlock.
* [`crates/wristkey-ui`](crates/wristkey-ui) *(Phase 4)*: Fluent Design Setup Wizard with real-time radar and distance calibration.

---

## Quick Start (Phase 1 CLI Prototype)

### Prerequisites

* Windows 11 or Windows 10 (version 1903+)
* Bluetooth 4.2+ adapter with LE support
* [Rust](https://rustup.rs/) (edition 2021+)

### 1. Build and Run Tests

```powershell
# Clone the repository
git clone https://github.com/jorjanion/WristKey.git
cd WristKey

# Run cryptographic and frame parsing tests
cargo test
```

### 2. Verify Cryptographic RPA Engine

```powershell
cargo run -p wristkey-cli -- --verify-crypto
```

Validates the `ah(IRK, prand)` implementation against the official Bluetooth SIG Core Specification test vectors.

### 3. Scan for Apple Devices in Real-Time

```powershell
cargo run -p wristkey-cli -- --scan
```

Outputs live BLE advertisements from nearby Apple devices, showing raw signal strength (RSSI), decoded Apple Continuity frame types, and on-wrist status.

### 4. Real-Time Proximity Calibration

```powershell
cargo run -p wristkey-cli -- --calibrate
```

Shows an animated visual signal meter with estimated distance in meters to help you fine-tune your lock and unlock thresholds:

```text
RSSI: -58.2 dBm [████████████░░░░░░░░] | Dist:  ~1.2m | State: NEAR (Unlock Range)
```

### 5. Inspect Windows Registry for Paired Watch IRKs

```powershell
cargo run -p wristkey-cli -- --list-keys
```

Queries `HKLM\SYSTEM\CurrentControlSet\Services\BTHPORT\Parameters\Keys` to dump paired Bluetooth LE Identity Resolving Keys (requires Administrator terminal or SYSTEM privileges).

### 6. Live Telemetry Inspector

```powershell
cargo run -p wristkey-cli -- --telemetry
```

Streams real-time decoded Apple Continuity bitfields: Action Code, On-Wrist flag, Passcode Locked state, Auto-Unlock capability, Wi-Fi status, and Auth Tags.

### 7. Configure Proximity Auto-Unlock (DPAPI Vault)

```powershell
# Set up login credentials encrypted with Windows DPAPI
cargo run -p wristkey-cli -- --setup-credentials

# Test vault encryption / decryption
cargo run -p wristkey-cli -- --test-vault
```

Credentials are encrypted using Windows Data Protection API (`CryptProtectData`), tied strictly to your Windows user account master key. No plaintext password is ever written to disk.

### 8. Enable Proximity Auto-Lock & Auto-Unlock

```powershell
cargo run -p wristkey-cli -- --auto-lock --auto-unlock --lock-threshold -82 --unlock-threshold -62
```

- **Away:** When you walk away (or remove the watch from your wrist), Windows locks immediately.
- **Approach:** When you return to your desk with your unlocked Apple Watch, WristKey wakes the lock screen and authorizes the Credential Provider via secure Named Pipe IPC (`\\.\pipe\WristKeyUnlockPipe`) to unlock your desktop seamlessly.

### 9. Launch Windows 11 Fluent Setup Wizard & Radar (GUI)

```powershell
# Launch the interactive Fluent UI (Tauri v2 + WebView2)
cargo run -p wristkey-ui
```

Features a step-by-step Setup Wizard with:
- **Radar Scanner:** Concentric animated BLE discovery for nearby Apple Watches.
- **Auto IRK Import:** One-click extraction from Windows Registry (`BTHPORT`).
- **Live Signal Calibrator:** Interactive sliders with real-time dBm meter to dial in desk vs room distances.
- **DPAPI Vault:** Seamless credential configuration.
- **Active Dashboard:** Real-time monitoring of signal, distance, on-wrist verification, and policies.

---

## Roadmap

- [x] **Fase 1:** BLE WinRT core scanner, Apple Continuity parser, RSSI EMA filter, auto-lock mechanism, and CLI.
- [x] **Fase 2:** Automated IRK extraction from Windows Registry (`HKLM\...\BTHPORT\Parameters\Keys`), on-wrist presence manager, and Apple Continuity telemetry.
- [x] **Fase 3:** Secure unlock via Windows Credential Provider V2, Named Pipe IPC, and DPAPI credential vault.
- [x] **Fase 4:** Fluent 2 Setup Wizard (Tauri v2) with visual radar calibrator and distance slider.

---

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE).

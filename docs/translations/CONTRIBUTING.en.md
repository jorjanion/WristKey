# Contributing to WristKey

Thank you for your interest in contributing to **WristKey**!  
WristKey is an open-source project and we warmly welcome contributions, whether they are bug reports, feature suggestions, or Pull Requests.

---

## 🐛 Reporting a Bug

Before opening an issue, please search the [existing Issues](https://github.com/jorjanion/WristKey/issues) to see if your problem has already been reported or resolved.

When submitting a bug report, please provide as much technical context as possible:

1. **Hardware & Environment:**
   - **Apple Watch Model:** (e.g., Apple Watch SE, Series 8, Ultra 2).
   - **watchOS Version:** (e.g., watchOS 10.6.2, 11.1).
   - **Windows Version:** (e.g., Windows 11 23H2 / 24H2, Windows 10 22H2).
   - **PC Bluetooth Adapter:** (e.g., Intel Wi-Fi 6E AX210, Realtek USB dongle).
   - **WristKey Version:** (e.g., v0.4.18).

2. **Diagnostic Telemetry (Highly Recommended):**
   If your issue relates to proximity detection, scanner visibility, or locking/unlocking, capture live BLE Continuity telemetry by running:
   ```powershell
   cargo run -p wristkey-cli -- --telemetry
   ```
   Paste a few relevant lines emitted by your watch (such as `Raw bytes: [10, 07, 3B, ...]`, RSSI in dBm) so we can accurately debug the Continuity status flags of your device.

3. **Expected vs Actual Behavior:**
   Explain what you expected to happen and what actually occurred.

---

## 💡 Suggesting Enhancements

If you have an idea to improve WristKey:
1. Open an issue using the **Feature Request** template.
2. Clearly describe the use case and expected benefit.

---

## 🛠️ Local Development & Pull Requests

### Prerequisites
* **Windows 10 / 11** with Bluetooth LE support.
* **Rust 2021+** (`rustup update`).
* **Node.js (v18+) & npm** (for the Tauri Fluent UI frontend).

### Development Workflow

1. **Fork and clone your repository:**
   ```powershell
   git clone https://github.com/<your-username>/WristKey.git
   cd WristKey
   ```

2. **Create a topic branch:**
   ```powershell
   git checkout -b feature/your-improvement-name
   # or
   git checkout -b fix/bug-description
   ```

3. **Run unit tests:**
   All changes must pass the workspace test suite:
   ```powershell
   cargo test --workspace
   ```

4. **Verify formatting and lints:**
   ```powershell
   cargo fmt --check
   cargo clippy --workspace -- -D warnings
   ```

5. **Test the UI in dev mode:**
   ```powershell
   npm --prefix crates/wristkey-ui run tauri dev
   ```

6. **Submit your Pull Request:**
   - Describe what problem it solves or feature it adds.
   - Reference open issues using `Closes #123` if applicable.

Thank you for supporting WristKey!

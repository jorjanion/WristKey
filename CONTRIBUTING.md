# Contributing to WristKey

<p align="right">
  🌐 <strong>Languages:</strong> 
  <strong>English (Main)</strong> | 
  <a href="docs/translations/CONTRIBUTING.es.md">Español</a> | 
  <a href="docs/translations/CONTRIBUTING.fr.md">Français</a> | 
  <a href="docs/translations/CONTRIBUTING.pt.md">Português</a> | 
  <a href="docs/translations/CONTRIBUTING.de.md">Deutsch</a> | 
  <a href="docs/translations/CONTRIBUTING.it.md">Italiano</a> | 
  <a href="docs/translations/CONTRIBUTING.zh-CN.md">简体中文</a> | 
  <a href="docs/translations/CONTRIBUTING.ko.md">한국어</a> | 
  <a href="docs/translations/CONTRIBUTING.ja.md">日本語</a>
</p>

Thank you for your interest in improving **WristKey**!  
WristKey is an open-source project and we warmly welcome contributions from the community, whether through bug reports, architectural ideas, or code Pull Requests.

---

## 🐛 Reporting a Bug

Before submitting a new issue, please review existing [GitHub Issues](https://github.com/jorjanion/WristKey/issues) to ensure the behavior hasn't already been addressed in a recent release.

When opening an issue, provide as much technical context as possible:

1. **Hardware & Environment:**
   - **Apple Watch Model:** (e.g. Apple Watch SE, Series 9, Ultra 2).
   - **watchOS Version:** (e.g. watchOS 10.6.2, 11.1).
   - **Windows Version:** (e.g. Windows 11 23H2 / 24H2, Windows 10 22H2).
   - **Bluetooth Adapter:** (e.g. Intel AX210, Realtek USB dongle, etc.).
   - **WristKey Version:** (e.g. v0.4.19).

2. **Diagnostics & Continuity Telemetry (Highly Recommended):**
   If reporting proximity tracking, packet scanning, or auto-lock/unlock issues, capture live telemetry in your terminal:
   ```powershell
   cargo run -p wristkey-cli -- --telemetry
   ```
   Paste a few relevant lines (e.g., `Raw bytes: [10, 07, 3B, ...]`, RSSI in dBm) so we can inspect your watch's exact Continuity flags.

3. **Observed vs Expected Behavior:**
   Explain what actually happened versus what you expected to happen.

---

## 💡 Proposing Enhancements (Feature Requests)

Have an idea to improve WristKey?
1. Open an issue using the **Feature Request** template.
2. Detail the motivation, workflow, and how it impacts end users.
3. If aligned with project architecture, we'll discuss implementation and milestones.

---

## 🛠️ Local Development & Pull Requests

### Prerequisites
* **Windows 10 / 11** with Bluetooth LE support.
* **Rust 2021+** (`rustup update`).
* **Node.js (v18+) and npm** (for the Tauri Fluent UI).

### Development Workflow

1. **Fork and clone the repository:**
   ```powershell
   git clone https://github.com/<your-username>/WristKey.git
   cd WristKey
   ```

2. **Create a topic branch:**
   ```powershell
   git checkout -b feature/your-feature-name
   # or
   git checkout -b fix/your-fix-name
   ```

3. **Run the workspace test suite:**
   All changes must pass existing unit, integration, and cryptographic tests:
   ```powershell
   cargo test --workspace
   ```

4. **Verify formatting & linting:**
   ```powershell
   cargo fmt --check
   cargo clippy --workspace -- -D warnings
   ```

5. **Test the frontend in development mode:**
   ```powershell
   npm --prefix crates/wristkey-ui run tauri dev
   ```

6. **Submit your Pull Request:**
   - Provide a concise summary of the changes made.
   - Reference any relevant issues (e.g., `Closes #123`).
   - Automated CI tests will validate your changes before merge.

Thank you for helping build a safer, faster desktop experience with WristKey!

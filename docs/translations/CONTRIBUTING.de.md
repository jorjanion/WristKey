# Mitwirken an WristKey / Contributing Guide

Vielen Dank für Ihr Interesse, **WristKey** zu unterstützen!

---

## 🐛 Fehler melden

Prüfen Sie vor dem Erstellen eines Issues bitte die [vorhandenen Issues](https://github.com/jorjanion/WristKey/issues).

Bitte geben Sie möglichst viele technische Details an:
1. **Apple Watch-Modell und watchOS-Version.**
2. **Windows-Version und Bluetooth-Adapter des PCs.**
3. **RF-Telemetrie (Sehr empfohlen):** Führen Sie `cargo run -p wristkey-cli -- --telemetry` aus und fügen Sie relevante Paketdaten ein.
4. **Erwartetes vs. tatsächliches Verhalten.**

---

## 🛠️ Lokale Entwicklung

1. **Fork klonen:**
   ```powershell
   git clone https://github.com/<ihr-benutzername>/WristKey.git
   cd WristKey
   ```
2. **Tests ausführen:**
   ```powershell
   cargo test --workspace
   ```
3. **Formatierung prüfen:**
   ```powershell
   cargo fmt --check
   cargo clippy --workspace -- -D warnings
   ```
4. **Pull Request einreichen** mit klarer Beschreibung der Änderungen.

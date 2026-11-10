<p align="center">
  <img src="../../assets/logo.png" width="112" height="112" alt="WristKey Logo" style="border-radius: 24px; box-shadow: 0 4px 20px rgba(0, 120, 212, 0.45);" />
</p>

<h1 align="center">WristKey</h1>

<p align="center">
  <strong>Automatische Näherungssperre & berührungsloses Entsperren für Windows 11 / 10 mit Apple Watch via BLE</strong><br>
  <em>Das nahtlose Auto-Unlock-Erlebnis von macOS nativ auf Windows übertragen.</em>
</p>

<p align="center">
  <a href="https://github.com/jorjanion/WristKey/releases/latest"><img src="https://img.shields.io/github/v/release/jorjanion/WristKey?color=2ea44f&label=Neueste%20Version" alt="Version"></a>
  <a href="../../LICENSE-MIT"><img src="https://img.shields.io/badge/Lizenz-MIT%20%2F%20Apache--2.0-blue.svg" alt="Lizenz"></a>
  <img src="https://img.shields.io/badge/Windows-11%20%7C%2010-0078D4?logo=windows&logoColor=white" alt="Windows">
  <img src="https://img.shields.io/badge/Rust-2021%2B-DEA584?logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/UI-Tauri%20v2-FFC131?logo=tauri&logoColor=black" alt="Tauri">
  <img src="https://img.shields.io/badge/Bluetooth-LE%204.2%2B-0082FC?logo=bluetooth&logoColor=white" alt="Bluetooth LE">
</p>

<p align="center">
  🌐 <strong>Sprachen / Languages:</strong><br>
  <a href="../../README.md">English (Main)</a> | <a href="README.es.md">Español</a> | 
  <a href="README.fr.md">Français</a> | 
  <a href="README.pt.md">Português</a> | 
  <strong>Deutsch</strong> | 
  <a href="README.it.md">Italiano</a> | 
  <a href="README.zh-CN.md">简体中文</a> | 
  <a href="README.ko.md">한국어</a> | 
  <a href="README.ja.md">日本語</a>
</p>

---

<div align="center">
  <img src="../../assets/dashboard_active_protection.png" alt="WristKey Fluent Dashboard" width="760" style="border-radius: 12px; border: 1px solid rgba(255,255,255,0.1); box-shadow: 0 8px 30px rgba(0,0,0,0.5);" />
  <p><em>Echtzeit-Schutzbereich · Kontinuierliche RSSI-Messung und optische Handgelenkserkennung</em></p>
</div>

---

## ⚡ Warum WristKey?

Windows bietet die Funktion *Dynamische Sperre* (Dynamic Lock), die jedoch im Alltag oft zu unzuverlässig ist. **WristKey** überwindet diese Einschränkungen durch echte Näherungsverfolgung basierend auf dem Apple-Continuity-Protokoll:

| Funktion | Windows Dynamische Sperre | WristKey |
| :--- | :--- | :--- |
| **Sperren beim Verlassen** | ❌ Träge (wartet 1–2 Minuten) | ✅ **Sofort & konfigurierbar** (asymmetrischer RSSI-Filter) |
| **Entsperren bei Rückkehr** | ❌ Nicht unterstützt (nur Sperre) | ✅ **Automatisches berührungsloses Entsperren** (Credential Provider) |
| **Optische Handgelenkprüfung** | ❌ Keine (bleibt auf Schreibtisch aktiv) | ✅ **Continuity 0x004C verifiziert** (Zugriff sofort widerrufen) |
| **Akkulaufzeit** | ❌ Hoch (ständiges klassisches Bluetooth) | ✅ **Extrem energieeffizient** (passive BLE-Advertisements) |
| **Adress-Datenschutz (RPA)** | ❌ Schlägt bei MAC-Rotation fehl | ✅ **Hardware-AES-128-IRK-Auflösung** (`ah(IRK, prand)`) |
| **Sicherheit der Anmeldedaten**| N/A | ✅ **Verschlüsselt mit Windows DPAPI** (niemals im Klartext) |

---

## 📥 Installation

Laden Sie das offizielle Installationsprogramm von der **[GitHub Releases](https://github.com/jorjanion/WristKey/releases)**-Seite herunter:
* 📦 **[WristKey_0.5.0_x64-setup.exe](https://github.com/jorjanion/WristKey/releases/download/v0.5.0/WristKey_0.5.0_x64-setup.exe)**

---

### ⚠️ Warum warnt Windows Defender SmartScreen?

Beim ersten Start des Installationsprogramms zeigt Microsoft Defender SmartScreen möglicherweise eine blaue Warnung an (*„Der Computer wurde durch Windows geschützt“*):

<div align="center">

| **Schritt 1:** Klick auf **„Weitere Informationen“** | **Schritt 2:** Klick auf **„Trotzdem ausführen“** |
| :---: | :---: |
| <img src="../../assets/smartscreen_step1.png" width="360" alt="Schritt 1" style="border-radius: 8px; border: 1px solid rgba(255,255,255,0.1);" /> | <img src="../../assets/smartscreen_step2.png" width="360" alt="Schritt 2" style="border-radius: 8px; border: 1px solid rgba(255,255,255,0.1);" /> |

</div>

#### Warum geschieht das?
1. **Fehlende Telemetrie-Reputation:** Neue Open-Source-Releases erzeugen einen neuen Binär-Hash, der in der Microsoft-Cloud noch keine tausendfachen Downloads verzeichnet hat.
2. **Kosten für EV-Zertifikate:** Microsoft verlangt für die automatische Warnungsfreiheit kommerzielle *EV Code Signing*-Zertifikate, die **300 bis 500 Dollar jährlich** kosten.
3. **100 % sicher und quelloffen:** Der Quellcode ist vollständig öffentlich auditierbar, Kennwörter werden über die offizielle **Windows DPAPI** verschlüsselt, und es gibt keine Tracker oder Drittanbieter-Telemetrie.

---

## 🤝 Mitwirken und Probleme melden

Details zur Fehlerberichterstattung und zu Pull Requests finden Sie in **[CONTRIBUTING.de.md](CONTRIBUTING.de.md)**.

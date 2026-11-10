<p align="center">
  <img src="../../assets/logo.png" width="112" height="112" alt="WristKey Logo" style="border-radius: 24px; box-shadow: 0 4px 20px rgba(0, 120, 212, 0.45);" />
</p>

<h1 align="center">WristKey</h1>

<p align="center">
  <strong>Blocco automatico di prossimità e sblocco a mani libere per Windows 11 / 10 con Apple Watch via BLE</strong><br>
  <em>La fluida esperienza di Auto Unlock di macOS portata nativamente su Windows.</em>
</p>

<p align="center">
  <a href="https://github.com/jorjanion/WristKey/releases/latest"><img src="https://img.shields.io/github/v/release/jorjanion/WristKey?color=2ea44f&label=Ultima%20versione" alt="Versione"></a>
  <a href="../../LICENSE-MIT"><img src="https://img.shields.io/badge/Licenza-MIT%20%2F%20Apache--2.0-blue.svg" alt="Licenza"></a>
  <img src="https://img.shields.io/badge/Windows-11%20%7C%2010-0078D4?logo=windows&logoColor=white" alt="Windows">
  <img src="https://img.shields.io/badge/Rust-2021%2B-DEA584?logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/UI-Tauri%20v2-FFC131?logo=tauri&logoColor=black" alt="Tauri">
  <img src="https://img.shields.io/badge/Bluetooth-LE%204.2%2B-0082FC?logo=bluetooth&logoColor=white" alt="Bluetooth LE">
</p>

<p align="center">
  🌐 <strong>Lingue / Languages:</strong><br>
  <a href="../../README.md">English (Main)</a> | <a href="README.es.md">Español</a> | 
  <a href="README.fr.md">Français</a> | 
  <a href="README.pt.md">Português</a> | 
  <a href="README.de.md">Deutsch</a> | 
  <strong>Italiano</strong> | 
  <a href="README.zh-CN.md">简体中文</a> | 
  <a href="README.ko.md">한국어</a> | 
  <a href="README.ja.md">日本語</a>
</p>

---

<div align="center">
  <img src="../../assets/dashboard_active_protection.png" alt="Dashboard WristKey Fluent" width="760" style="border-radius: 12px; border: 1px solid rgba(255,255,255,0.1); box-shadow: 0 8px 30px rgba(0,0,0,0.5);" />
  <p><em>Pannello di Protezione Attiva in tempo reale · Monitoraggio continuo RSSI e rilevamento ottico al polso</em></p>
</div>

---

## ⚡ Perché WristKey?

Windows include una funzione denominata *Blocco dinamico* (Dynamic Lock), ma presenta forti limitazioni. **WristKey** risolve queste criticità implementando un monitoraggio continuo tramite l'analisi del protocollo Apple Continuity:

| Funzionalità | Windows Dynamic Lock | WristKey |
| :--- | :--- | :--- |
| **Blocco all'allontanamento** | ❌ Lento (attende da 1 a 2 minuti) | ✅ **Immediato e regolabile** (filtro RSSI asimmetrico) |
| **Sblocco all'avvicinamento** | ❌ Non supportato (solo blocco) | ✅ **Sblocco automatico a mani libere** (Credential Provider) |
| **Rilevamento ottico al polso** | ❌ Nessuno (rimane attivo sul tavolo) | ✅ **Verificato con Continuity 0x004C** (revoca immediata se sfilato) |
| **Consumo di batteria** | ❌ Elevato (connessione Bluetooth classica) | ✅ **Bassissimo consumo** (pacchetti passivi BLE) |
| **Privacy indirizzo (RPA)** | ❌ Fallisce alla rotazione del MAC BLE | ✅ **Risoluzione hardware AES-128 IRK** (`ah(IRK, prand)`) |
| **Sicurezza credenziali** | N/A | ✅ **Crittografia con Windows DPAPI** (mai in chiaro su disco) |

---

## 📥 Installazione

Scarica il programma di installazione ufficiale da **[GitHub Releases](https://github.com/jorjanion/WristKey/releases)**:
* 📦 **[WristKey_0.4.19_x64-setup.exe](https://github.com/jorjanion/WristKey/releases/download/v0.4.19/WristKey_0.4.19_x64-setup.exe)**

---

### ⚠️ Perché Windows Defender SmartScreen mostra un avviso?

Al primo avvio, Windows Defender SmartScreen potrebbe mostrare una schermata blu (*"PC protetto da Windows"*):

<div align="center">

| **Passo 1:** Clicca su **"Ulteriori informazioni"** | **Passo 2:** Clicca su **"Esegui comunque"** |
| :---: | :---: |
| <img src="../../assets/smartscreen_step1.png" width="360" alt="Passo 1" style="border-radius: 8px; border: 1px solid rgba(255,255,255,0.1);" /> | <img src="../../assets/smartscreen_step2.png" width="360" alt="Passo 2" style="border-radius: 8px; border: 1px solid rgba(255,255,255,0.1);" /> |

</div>

#### Il motivo:
1. **Mancanza di reputazione nel cloud Microsoft:** Ogni nuova release genera un hash SHA-256 inedito che non ha ancora totalizzato migliaia di download.
2. **Costo dei certificati EV:** I certificati commerciali per evitare gli avvisi costano **tra $300 e $500 all'anno**, una spesa non sostenibile per software open-source indipendente gratuito.
3. **100% Open Source e Sicuro:** Il codice sorgente è verificabile in questo repository, non contiene telemetria di terze parti e le password sono protette con **Windows DPAPI**.

---

## 🤝 Contribuire

Consulta il file **[CONTRIBUTING.it.md](CONTRIBUTING.it.md)** per le linee guida di segnalazione bug e invio di Pull Request.

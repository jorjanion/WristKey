<p align="center">
  <img src="assets/logo.png" width="112" height="112" alt="WristKey Logo" style="border-radius: 24px; box-shadow: 0 4px 20px rgba(0, 120, 212, 0.45);" />
</p>

<h1 align="center">WristKey</h1>

<p align="center">
  <strong>Proximity Auto-Lock & Hands-Free Unlock for Windows 11 / 10 using Apple Watch via BLE</strong><br>
  <em>La experiencia fluida de Auto Unlock de macOS traída de forma nativa a Windows.</em>
</p>

<p align="center">
  <a href="https://github.com/jorjanion/WristKey/releases/latest"><img src="https://img.shields.io/github/v/release/jorjanion/WristKey?color=2ea44f&label=Latest%20Release" alt="Release"></a>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/License-MIT%20%2F%20Apache--2.0-blue.svg" alt="License"></a>
  <img src="https://img.shields.io/badge/Windows-11%20%7C%2010-0078D4?logo=windows&logoColor=white" alt="Windows">
  <img src="https://img.shields.io/badge/Rust-2021%2B-DEA584?logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/UI-Tauri%20v2-FFC131?logo=tauri&logoColor=black" alt="Tauri">
  <img src="https://img.shields.io/badge/Bluetooth-LE%204.2%2B-0082FC?logo=bluetooth&logoColor=white" alt="Bluetooth LE">
</p>

<p align="center">
  🌐 <strong>Idiomas / Languages:</strong><br>
  <strong>Español (Principal)</strong> | 
  <a href="docs/translations/README.en.md">English</a> | 
  <a href="docs/translations/README.fr.md">Français</a> | 
  <a href="docs/translations/README.pt.md">Português</a> | 
  <a href="docs/translations/README.de.md">Deutsch</a> | 
  <a href="docs/translations/README.it.md">Italiano</a> | 
  <a href="docs/translations/README.zh-CN.md">简体中文</a> | 
  <a href="docs/translations/README.ko.md">한국어</a> | 
  <a href="docs/translations/README.ja.md">日本語</a>
</p>

---

<div align="center">
  <img src="assets/dashboard_active_protection.png" alt="WristKey Fluent Dashboard" width="760" style="border-radius: 12px; border: 1px solid rgba(255,255,255,0.1); box-shadow: 0 8px 30px rgba(0,0,0,0.5);" />
  <p><em>Panel de Protección Activa en tiempo real · Medición RSSI continua y verificación óptica en muñeca</em></p>
</div>

---

## ⚡ ¿Por qué WristKey? / Why WristKey?

Windows incluye una función llamada *Bloqueo dinámico* (Dynamic Lock), pero resulta insuficiente en el día a día. **WristKey** supera estas limitaciones implementando seguimiento de proximidad real mediante el análisis del protocolo nativo Apple Continuity:

| Característica / Feature | Windows Dynamic Lock | WristKey |
| :--- | :--- | :--- |
| **Auto-bloqueo al alejarte** | ❌ Lento (espera hasta 1 o 2 minutos) | ✅ **Inmediato y configurable** (filtro RSSI asimétrico) |
| **Auto-desbloqueo al acercarte** | ❌ No soportado (solo bloquea) | ✅ **Desbloqueo automático manos libres** (Credential Provider) |
| **Verificación óptica en muñeca** | ❌ Nula (sigue activo con reloj en mesa) | ✅ **Continuity 0x004C verificado** (revoca acceso al quitártelo) |
| **Consumo de batería** | ❌ Alto (Bluetooth clásico constante) | ✅ **Ultra bajo consumo** (anuncios pasivos Bluetooth LE) |
| **Privacidad de dirección (RPA)** | ❌ Falla cuando la MAC de BLE rota | ✅ **Resolución hardware AES-128 IRK** (`ah(IRK, prand)`) |
| **Seguridad de credenciales** | N/A | ✅ **Cifrado local con Windows DPAPI** (nunca en texto plano) |

---

Similar a la funcionalidad nativa de **Auto Unlock en macOS** y a **`watch-unlock-rs` en Linux**, **WristKey** monitoriza continuamente en segundo plano tu Apple Watch a través de Bluetooth Low Energy (BLE):
- 🔒 **Bloquea automáticamente tu PC** cuando te levantas o te alejas de tu escritorio.
- 🔓 **Enciende la pantalla y desbloquea tu sesión** al regresar a tu puesto de trabajo.
- 🛑 **Seguridad óptica en muñeca:** Si te quitas el reloj o este se bloquea con código, el acceso se revoca de inmediato.

---

## 📥 Instalación / Installation

Descarga el instalador oficial desde la sección de **[GitHub Releases](https://github.com/jorjanion/WristKey/releases)**:
* 📦 **[WristKey_0.4.18_x64-setup.exe](https://github.com/jorjanion/WristKey/releases/download/v0.4.18/WristKey_0.4.18_x64-setup.exe)**

Ejecuta el archivo `.exe` para instalar WristKey en tu equipo con su acceso directo y servicio en segundo plano.

---

### ⚠️ ¿Por qué Windows SmartScreen muestra un aviso al instalar?

Al ejecutar el instalador por primera vez en Windows 11 o 10, es posible que veas la pantalla azul de aviso de **Microsoft Defender SmartScreen** (*"Windows protegió su PC"*):

<div align="center">

| **Paso 1:** Haz clic en **"Más información"** | **Paso 2:** Pulsa en **"Ejecutar de todas formas"** |
| :---: | :---: |
| <img src="assets/smartscreen_step1.png" width="360" alt="Paso 1: Más información" style="border-radius: 8px; border: 1px solid rgba(255,255,255,0.1);" /> | <img src="assets/smartscreen_step2.png" width="360" alt="Paso 2: Ejecutar de todas formas" style="border-radius: 8px; border: 1px solid rgba(255,255,255,0.1);" /> |

</div>

#### ¿Por qué ocurre esto?
1. **Falta de reputación en la nube de Microsoft:** SmartScreen es un filtro basado en telemetría de Microsoft. Cada versión nueva (`v0.4.18`, etc.) genera un binario nuevo con un hash criptográfico único que la nube de Microsoft aún no ha visto miles de veces descargado.
2. **Coste de los certificados de firma de código (EV):** Para que Windows no muestre advertencias en aplicaciones de escritorio recién creadas, Microsoft exige que los desarrolladores adquieran un certificado *EV Code Signing* que cuesta entre **$300 y $500 al año** mediante empresas de certificación corporativas (DigiCert, Sectigo). En proyectos comunitarios y de código abierto independientes, este gasto no es viable.
3. **Seguridad y Privacidad 100% Verificable:**
   - WristKey es **100% código abierto** (Open Source).
   - Puedes auditar todo el código fuente en Rust en este mismo repositorio.
   - Tus contraseñas se almacenan cifradas localmente con **Windows DPAPI** (`CryptProtectData`) vinculadas a tu cuenta de usuario.
   - No contiene telemetría de terceros ni conexiones a servidores externos.

#### ¿Cómo continuar con la instalación?
1. En la primera pantalla, haz clic en el enlace subrayado **"Más información"** (*"More info"*).
2. Aparecerá el botón adicional en la parte inferior izquierda; pulsa en **"Ejecutar de todas formas"** (*"Run anyway"*).

---

## Arquitectura y Seguridad / Architecture & Security

1. **RPA Resolution (AES-128 IRK):**
   Apple devices rotate their Bluetooth MAC address every ~15 minutes (Resolvable Private Address). WristKey resolves RPAs against the paired watch's Identity Resolving Key (IRK) using hardware AES-128 (`ah(IRK, prand)` per Bluetooth SIG Core Spec).
2. **Apple Continuity Telemetry (0x004C):**
   Parses Apple Continuity frames (`type 0x10 Nearby Info`). If you take off the watch or lock it, watchOS asserts the lock flag; WristKey denies access until re-authenticated on wrist.
3. **DPAPI Credential Encryption:**
   Login credentials used for automatic unlock are stored using Windows DPAPI (`CryptProtectData`), encrypted with your user account master key. Plaintext passwords are not stored on disk.
4. **Named Pipe IPC:**
   Authentication between the daemon and the Windows Credential Provider communicates over a restricted local Named Pipe (`\\.\pipe\WristKeyUnlockPipe`).

```
Apple Watch (BLE Advertisement)
  │  (RPA rotated every 15 min + Apple Continuity telemetry)
  ▼
Windows BLE Watcher (WinRT)
  ▼
RPA Resolver (AES-128 IRK matching via Bluetooth Core Spec)
  ▼
Apple Continuity Parser (0x004C / 0x10 Nearby Info)
  │  Verifies watch is on wrist and unlocked (Action Code 0x0A / 0x0B)
  ▼
RSSI Asymmetric EMA Filter & Hysteresis State Machine
  ├── Walk Away (RSSI < -80 dBm or Off-Wrist)  ──> LockWorkStation()
  └── Approach  (RSSI > -58 dBm and On-Wrist)  ──> Wake Screen & Unlock via Credential Provider
```

---

## CLI Usage

The repository includes a command-line tool (`wristkey-cli`) for diagnostics, signal calibration, packet inspection, and headless background execution.

### Building from Source

Prerequisites:
* Windows 11 or Windows 10 (version 1903+)
* Bluetooth 4.2+ adapter with LE support
* Rust (edition 2021+)

```powershell
# Clone the repository
git clone https://github.com/jorjanion/WristKey.git
cd WristKey

# Run unit and cryptographic tests
cargo test
```

### Commands & Diagnostics

#### 1. Real-Time Packet Scanner (`--scan`)
Scans for nearby BLE advertisements and decodes Apple Continuity payloads (`0x004C`), showing RSSI, frame types, and raw bytes:
```powershell
cargo run -p wristkey-cli -- --scan
```

#### 2. Signal & Distance Calibrator (`--calibrate`)
Displays a live visual RSSI signal meter and estimated distance in meters. Use this to determine optimal lock and unlock thresholds for your workspace:
```powershell
cargo run -p wristkey-cli -- --calibrate
```

#### 3. Extract Paired IRK Keys (`--list-keys`)
Queries `HKLM\SYSTEM\CurrentControlSet\Services\BTHPORT\Parameters\Keys` to dump paired Bluetooth LE Identity Resolving Keys (requires Administrator terminal):
```powershell
cargo run -p wristkey-cli -- --list-keys
```

#### 4. Verify Cryptographic Engine (`--verify-crypto`)
Validates the local `ah(IRK, prand)` AES-128 implementation against the official Bluetooth SIG Core Specification test vectors:
```powershell
cargo run -p wristkey-cli -- --verify-crypto
```

#### 5. Continuity Telemetry Inspector (`--telemetry`)
Real-time decoding of Nearby Info flags (on-wrist state, user activity, auto-unlock eligibility, Wi-Fi status, and auth tags):
```powershell
cargo run -p wristkey-cli -- --telemetry
```

---

## 🤝 Contribuir y Reportar Errores / Contributing & Issues

¡Damos la bienvenida a cualquier contribución de la comunidad! Puedes colaborar de las siguientes formas:

### 1. Reportar un Bug / Abrir un Issue
Si encuentras un comportamiento inesperado o tu reloj no se detecta adecuadamente:
1. Dirígete a la pestaña de **[GitHub Issues](https://github.com/jorjanion/WristKey/issues/new/choose)**.
2. Selecciona la plantilla de **Reporte de Error** e incluye:
   - Modelo exacto de Apple Watch y versión de watchOS (ej. Apple Watch SE, watchOS 10.6.2).
   - Versión de Windows y modelo de adaptador Bluetooth del PC.
   - Salida del comando de telemetría: `cargo run -p wristkey-cli -- --telemetry`.
   - Comportamiento observado vs comportamiento esperado.

### 2. Proponer una Mejora
Si tienes una idea para una nueva característica, abre un issue con la plantilla **Solicitud de Función / Feature Request**.

### 3. Enviar un Pull Request (PR)
1. Haz un fork del repositorio y crea una rama para tu cambio (`git checkout -b feature/nombre-mejora`).
2. Verifica que toda la suite de pruebas unitarias pase: `cargo test --workspace`.
3. Comprueba el formato de código: `cargo fmt --check`.
4. Abre tu Pull Request en GitHub describiendo detalladamente la solución.

Para una guía paso a paso completa, consulta el archivo **[CONTRIBUTING.md](CONTRIBUTING.md)**.

---

## Troubleshooting

1. **Bluetooth Adapter Not Recognized:** Ensure your PC's Bluetooth adapter supports Bluetooth Low Energy (BLE 4.2 or higher).
2. **Wrist Detection Disabled on Watch:** In the Apple Watch app on your iPhone, go to *Passcode* and ensure *Wrist Detection* is toggled **ON**.
3. **Display Does Not Wake:** Ensure Windows Power & Sleep settings allow peripheral wake events.

---

## License

Licensed under either of:
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
* MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

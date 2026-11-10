<p align="center">
  <img src="../../assets/logo.png" width="112" height="112" alt="WristKey Logo" style="border-radius: 24px; box-shadow: 0 4px 20px rgba(0, 120, 212, 0.45);" />
</p>

<h1 align="center">WristKey</h1>

<p align="center">
  <strong>Proximity Auto-Lock & Hands-Free Unlock for Windows 11 / 10 using Apple Watch via BLE</strong><br>
  <em>La experiencia fluida de Auto Unlock de macOS traída de forma nativa a Windows.</em>
</p>

<p align="center">
  <a href="https://github.com/jorjanion/WristKey/releases/latest"><img src="https://img.shields.io/github/v/release/jorjanion/WristKey?color=2ea44f&label=Latest%20Release" alt="Release"></a>
  <a href="../../LICENSE-MIT"><img src="https://img.shields.io/badge/License-MIT%20%2F%20Apache--2.0-blue.svg" alt="License"></a>
  <img src="https://img.shields.io/badge/Windows-11%20%7C%2010-0078D4?logo=windows&logoColor=white" alt="Windows">
  <img src="https://img.shields.io/badge/Rust-2021%2B-DEA584?logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/UI-Tauri%20v2-FFC131?logo=tauri&logoColor=black" alt="Tauri">
  <img src="https://img.shields.io/badge/Bluetooth-LE%204.2%2B-0082FC?logo=bluetooth&logoColor=white" alt="Bluetooth LE">
</p>

<p align="center">
  🌐 <strong>Idiomas / Languages:</strong><br>
  <a href="../../README.md">English (Main)</a> | 
  <strong>Español</strong> | 
  <a href="README.fr.md">Français</a> | 
  <a href="README.pt.md">Português</a> | 
  <a href="README.de.md">Deutsch</a> | 
  <a href="README.it.md">Italiano</a> | 
  <a href="README.zh-CN.md">简体中文</a> | 
  <a href="README.ko.md">한국어</a> | 
  <a href="README.ja.md">日本語</a>
</p>

---

<div align="center">
  <img src="../../assets/dashboard_active_protection.png" alt="WristKey Fluent Dashboard" width="760" style="border-radius: 12px; border: 1px solid rgba(255,255,255,0.1); box-shadow: 0 8px 30px rgba(0,0,0,0.5);" />
  <p><em>Panel de Protección Activa en tiempo real · Medición RSSI continua y verificación óptica en muñeca</em></p>
</div>

---

## ⚡ ¿Por qué WristKey?

Windows incluye una función llamada *Bloqueo dinámico* (Dynamic Lock), pero resulta insuficiente en el día a día. **WristKey** supera estas limitaciones implementando seguimiento de proximidad real mediante el análisis del protocolo nativo Apple Continuity:

| Característica | Windows Dynamic Lock | WristKey |
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

## 📥 Instalación

Descarga el instalador oficial desde la sección de **[GitHub Releases](https://github.com/jorjanion/WristKey/releases)**:
* 📦 **[WristKey_0.4.19_x64-setup.exe](https://github.com/jorjanion/WristKey/releases/download/v0.4.19/WristKey_0.4.19_x64-setup.exe)**

Ejecuta el archivo `.exe` para instalar WristKey en tu equipo con su acceso directo y servicio en segundo plano. Permite seleccionar tu idioma predilecto durante la instalación y cambiarlo en cualquier momento desde la app.

---

### ⚠️ ¿Por qué Windows SmartScreen muestra un aviso al instalar?

Al ejecutar el instalador por primera vez en Windows 11 o 10, es posible que veas la pantalla azul de aviso de **Microsoft Defender SmartScreen** (*"Windows protegió su PC"*):

<div align="center">

| **Paso 1:** Haz clic en **"Más información"** | **Paso 2:** Pulsa en **"Ejecutar de todas formas"** |
| :---: | :---: |
| <img src="../../assets/smartscreen_step1.png" width="360" alt="Paso 1: Más información" style="border-radius: 8px; border: 1px solid rgba(255,255,255,0.1);" /> | <img src="../../assets/smartscreen_step2.png" width="360" alt="Paso 2: Ejecutar de todas formas" style="border-radius: 8px; border: 1px solid rgba(255,255,255,0.1);" /> |

</div>

#### ¿Por qué ocurre esto?
1. **Falta de reputación en la nube de Microsoft:** SmartScreen es un filtro basado en telemetría de Microsoft. Cada versión nueva (`v0.4.19`, etc.) genera un binario nuevo con un hash criptográfico único que la nube de Microsoft aún no ha visto miles de veces descargado.
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

## Arquitectura y Seguridad

1. **Resolución RPA (AES-128 IRK):**
   Los dispositivos Apple rotan su dirección MAC de Bluetooth cada ~15 minutos (Resolvable Private Address). WristKey resuelve los RPAs contra la Clave de Resolución de Identidad (IRK) del reloj emparejado utilizando AES-128 por hardware (`ah(IRK, prand)` según la especificación Bluetooth SIG Core).
2. **Telemetría Apple Continuity (0x004C):**
   Decodifica tramas Apple Continuity (`tipo 0x10 Nearby Info`). Si te quitas el reloj o lo bloqueas con código, watchOS marca el indicador de bloqueo; WristKey deniega el acceso hasta que vuelva a ser colocado y desbloqueado en la muñeca.
3. **Cifrado de Credenciales con DPAPI:**
   Las credenciales de inicio de sesión se almacenan utilizando Windows DPAPI (`CryptProtectData`), cifradas con la clave maestra de tu usuario. No se almacenan contraseñas en texto plano.
4. **IPC mediante Named Pipes:**
   La autenticación entre el demonio y el Windows Credential Provider se comunica a través de una canalización local Named Pipe restringida (`\\.\pipe\WristKeyUnlockPipe`).

```
Apple Watch (Emisión BLE)
  │  (RPA rotada cada 15 min + telemetría Apple Continuity)
  ▼
Windows BLE Watcher (WinRT)
  ▼
RPA Resolver (Emparejamiento AES-128 IRK vía Bluetooth Core Spec)
  ▼
Apple Continuity Parser (0x004C / 0x10 Nearby Info)
  │  Verifica que el reloj está puesto en muñeca y desbloqueado (Action Code 0x0A / 0x0B)
  ▼
Filtro Asimétrico EMA RSSI & Máquina de Estados con Histéresis
  ├── Alejarse (RSSI < -80 dBm o Fuera de muñeca)  ──> LockWorkStation()
  └── Acercarse (RSSI > -58 dBm y En muñeca)       ──> Despertar pantalla y desbloquear
```

---

## Uso de la CLI

El repositorio incluye una herramienta por línea de comandos (`wristkey-cli`) para diagnósticos, calibración de señal, inspección de paquetes y ejecución en segundo plano.

### Compilar desde el código fuente

Requisitos:
* Windows 11 o Windows 10 (versión 1903+)
* Adaptador Bluetooth 4.2+ con soporte LE
* Rust (edición 2021+)

```powershell
# Clonar el repositorio
git clone https://github.com/jorjanion/WristKey.git
cd WristKey

# Ejecutar las pruebas unitarias y criptográficas
cargo test
```

### Comandos y Diagnósticos

#### 1. Escáner de Paquetes en Tiempo Real (`--scan`)
```powershell
cargo run -p wristkey-cli -- --scan
```

#### 2. Calibrador de Señal y Distancia (`--calibrate`)
```powershell
cargo run -p wristkey-cli -- --calibrate
```

#### 3. Extraer Claves IRK Emparejadas (`--list-keys`)
```powershell
cargo run -p wristkey-cli -- --list-keys
```

#### 4. Verificar Motor Criptográfico (`--verify-crypto`)
```powershell
cargo run -p wristkey-cli -- --verify-crypto
```

#### 5. Inspector de Telemetría Continuity (`--telemetry`)
```powershell
cargo run -p wristkey-cli -- --telemetry
```

---

## 🤝 Contribuir y Reportar Errores

¡Damos la bienvenida a cualquier contribución de la comunidad! Consulta el archivo **[CONTRIBUTING.es.md](CONTRIBUTING.es.md)** para más detalles sobre cómo abrir issues o enviar Pull Requests.

---

## Licencia

Distribuido bajo cualquiera de las siguientes licencias:
* Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE))
* MIT License ([LICENSE-MIT](../../LICENSE-MIT))

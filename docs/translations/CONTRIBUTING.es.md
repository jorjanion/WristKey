# Guía de Contribución

<p align="right">
  🌐 <strong>Idiomas:</strong> 
  <a href="../../CONTRIBUTING.md">English (Main)</a> | 
  <strong>Español</strong> | 
  <a href="CONTRIBUTING.fr.md">Français</a> | 
  <a href="CONTRIBUTING.pt.md">Português</a> | 
  <a href="CONTRIBUTING.de.md">Deutsch</a> | 
  <a href="CONTRIBUTING.it.md">Italiano</a> | 
  <a href="CONTRIBUTING.zh-CN.md">简体中文</a> | 
  <a href="CONTRIBUTING.ko.md">한국어</a> | 
  <a href="CONTRIBUTING.ja.md">日本語</a>
</p>

¡Gracias por tu interés en mejorar **WristKey**!  
WristKey es un proyecto open-source y damos la bienvenida a contribuciones de la comunidad, tanto reportes de errores como sugerencias y Pull Requests.

---

## 🐛 Reportar un Error

Antes de abrir un issue, revisa la lista de [Issues existentes](https://github.com/jorjanion/WristKey/issues) para verificar si el problema ya ha sido reportado o resuelto en versiones anteriores.

Al crear un nuevo issue, incluye la mayor cantidad de contexto técnico posible:

1. **Hardware y Software:**
   - **Modelo de Apple Watch:** (ej. Apple Watch SE, Series 8, Ultra 2).
   - **Versión de watchOS:** (ej. watchOS 10.6.2, 11.1).
   - **Versión de Windows:** (ej. Windows 11 23H2 / 24H2, Windows 10 22H2).
   - **Adaptador Bluetooth del PC:** (ej. Intel Wi-Fi 6E AX210 integrado, dongle USB Realtek, etc.).
   - **Versión de WristKey:** (ej. v0.4.19).

2. **Diagnóstico y Telemetría RF (Muy recomendado):**
   Si el problema está relacionado con la detección de proximidad, el escáner o el bloqueo/desbloqueo, puedes capturar la telemetría en vivo ejecutando en una terminal:
   ```powershell
   cargo run -p wristkey-cli -- --telemetry
   ```
   Copia unas pocas líneas relevantes del paquete emitido por tu reloj (por ejemplo: `Raw bytes: [10, 07, 3B, ...]`, RSSI en dBm) para que podamos depurar exactamente los flags de Continuity de tu dispositivo.

3. **Comportamiento esperado vs Comportamiento real:**
   Explica qué esperabas que ocurriera y qué ocurrió exactamente (ej. "Al alejarme más de 3 metros, el medidor bajó a -82 dBm pero la pantalla no se bloqueó").

---

## 💡 Sugerir una Mejora

Si tienes una idea para mejorar WristKey:
1. Abre un issue utilizando la plantilla de **Feature Request**.
2. Explica claramente el caso de uso y el beneficio para los usuarios.
3. Si es factible y aporta valor, la priorizaremos para las siguientes versiones.

---

## 🛠️ Desarrollo Local y Pull Requests

### Requisitos Previos
* **Windows 10 / 11** con soporte Bluetooth LE.
* **Rust 2021+** (`rustup update`).
* **Node.js (v18+) y npm** (para la interfaz Tauri Fluent UI).

### Pasos para Desarrollar

1. **Haz un Fork y clona tu repositorio:**
   ```powershell
   git clone https://github.com/<tu-usuario>/WristKey.git
   cd WristKey
   ```

2. **Crea una rama para tu feature o fix:**
   ```powershell
   git checkout -b feature/nombre-de-tu-mejora
   # o
   git checkout -b fix/descripcion-del-bug
   ```

3. **Ejecuta los tests unitarios:**
   Todos los cambios deben pasar la suite de pruebas del espacio de trabajo:
   ```powershell
   cargo test --workspace
   ```

4. **Verifica el formato del código:**
   ```powershell
   cargo fmt --check
   cargo clippy --workspace -- -D warnings
   ```

5. **Prueba la interfaz en modo desarrollo:**
   ```powershell
   npm --prefix crates/wristkey-ui run tauri dev
   ```

6. **Envía tu Pull Request:**
   - Asegúrate de describir claramente qué problema resuelve o qué función añade.
   - Si tu PR resuelve un issue abierto, añade `Closes #123` en la descripción.
   - Un mantenedor revisará los cambios y ejecutará la integración continua (CI) en GitHub Actions antes de fusionar.

¡Gracias por apoyar el ecosistema open-source de WristKey!

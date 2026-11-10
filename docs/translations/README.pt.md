<p align="center">
  <img src="../../assets/logo.png" width="112" height="112" alt="WristKey Logo" style="border-radius: 24px; box-shadow: 0 4px 20px rgba(0, 120, 212, 0.45);" />
</p>

<h1 align="center">WristKey</h1>

<p align="center">
  <strong>Bloqueio automático por proximidade e desbloqueio sem mãos para Windows 11 / 10 com Apple Watch via BLE</strong><br>
  <em>A experiência fluida do Auto Unlock do macOS portada nativamente para o Windows.</em>
</p>

<p align="center">
  <a href="https://github.com/jorjanion/WristKey/releases/latest"><img src="https://img.shields.io/github/v/release/jorjanion/WristKey?color=2ea44f&label=%C3%9Altima%20Vers%C3%A3o" alt="Versão"></a>
  <a href="../../LICENSE-MIT"><img src="https://img.shields.io/badge/Licen%C3%A7a-MIT%20%2F%20Apache--2.0-blue.svg" alt="Licença"></a>
  <img src="https://img.shields.io/badge/Windows-11%20%7C%2010-0078D4?logo=windows&logoColor=white" alt="Windows">
  <img src="https://img.shields.io/badge/Rust-2021%2B-DEA584?logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/UI-Tauri%20v2-FFC131?logo=tauri&logoColor=black" alt="Tauri">
  <img src="https://img.shields.io/badge/Bluetooth-LE%204.2%2B-0082FC?logo=bluetooth&logoColor=white" alt="Bluetooth LE">
</p>

<p align="center">
  🌐 <strong>Idiomas / Languages:</strong><br>
  <a href="../../README.md">English (Main)</a> | <a href="README.es.md">Español</a> | 
  <a href="README.fr.md">Français</a> | 
  <strong>Português</strong> | 
  <a href="README.de.md">Deutsch</a> | 
  <a href="README.it.md">Italiano</a> | 
  <a href="README.zh-CN.md">简体中文</a> | 
  <a href="README.ko.md">한국어</a> | 
  <a href="README.ja.md">日本語</a>
</p>

---

<div align="center">
  <img src="../../assets/dashboard_active_protection.png" alt="Painel WristKey Fluent" width="760" style="border-radius: 12px; border: 1px solid rgba(255,255,255,0.1); box-shadow: 0 8px 30px rgba(0,0,0,0.5);" />
  <p><em>Painel de Proteção Ativa em tempo real · Medição RSSI contínua e verificação óptica no pulso</em></p>
</div>

---

## ⚡ Por que WristKey?

O Windows inclui um recurso chamado *Bloqueio Dinâmico* (Dynamic Lock), mas ele não atende bem às necessidades diárias. O **WristKey** supera essas limitações implementando rastreamento de proximidade autêntico via protocolo Apple Continuity:

| Recurso | Windows Dynamic Lock | WristKey |
| :--- | :--- | :--- |
| **Bloqueio ao afastar-se** | ❌ Lento (espera até 1 a 2 minutos) | ✅ **Imediato e ajustável** (filtro RSSI assimétrico) |
| **Desbloqueio ao aproximar-se** | ❌ Não suportado (apenas bloqueia) | ✅ **Desbloqueio mãos-livres** (Credential Provider) |
| **Verificação óptica no pulso** | ❌ Nula (permanece ativo na mesa) | ✅ **Validado por Continuity 0x004C** (revoga acesso se removido) |
| **Consumo de bateria** | ❌ Alto (Bluetooth clássico constante) | ✅ **Ultra baixo consumo** (anúncios passivos BLE) |
| **Privacidade de endereço (RPA)** | ❌ Falha na rotação de endereço MAC | ✅ **Resolução por hardware AES-128 IRK** (`ah(IRK, prand)`) |
| **Segurança de credenciais** | N/A | ✅ **Criptografado com Windows DPAPI** (nunca em texto simples) |

---

## 📥 Instalação

Baixe o instalador oficial na página de **[GitHub Releases](https://github.com/jorjanion/WristKey/releases)**:
* 📦 **[WristKey_0.4.19_x64-setup.exe](https://github.com/jorjanion/WristKey/releases/download/v0.4.19/WristKey_0.4.19_x64-setup.exe)**

---

### ⚠️ Por que o Windows SmartScreen exibe um aviso?

Ao executar o instalador pela primeira vez, o Windows Defender SmartScreen pode exibir um aviso azul (*"O Windows protegeu o seu computador"*):

<div align="center">

| **Passo 1:** Clique em **"Mais informações"** | **Passo 2:** Clique em **"Executar assim mesmo"** |
| :---: | :---: |
| <img src="../../assets/smartscreen_step1.png" width="360" alt="Passo 1" style="border-radius: 8px; border: 1px solid rgba(255,255,255,0.1);" /> | <img src="../../assets/smartscreen_step2.png" width="360" alt="Passo 2" style="border-radius: 8px; border: 1px solid rgba(255,255,255,0.1);" /> |

</div>

#### Motivo:
1. **Falta de reputação na nuvem da Microsoft:** Cada nova compilação gera um hash SHA-256 inédito que ainda não acumulou milhares de downloads.
2. **Custo dos certificados EV:** Certificados corporativos de assinatura de código custam **$300 a $500 anuais**, algo inviável para software comunitário livre.
3. **100% Código Aberto e Seguro:** O WristKey é auditável, suas senhas são protegidas com a API oficial **Windows DPAPI** e não há qualquer telemetria externa.

---

## 🤝 Contribuir e Relatar Problemas

Consulte o arquivo **[CONTRIBUTING.pt.md](CONTRIBUTING.pt.md)** para instruções sobre como reportar bugs e enviar Pull Requests.

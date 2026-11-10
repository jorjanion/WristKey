<p align="center">
  <img src="../../assets/logo.png" width="112" height="112" alt="WristKey Logo" style="border-radius: 24px; box-shadow: 0 4px 20px rgba(0, 120, 212, 0.45);" />
</p>

<h1 align="center">WristKey</h1>

<p align="center">
  <strong>基于 BLE 的 Apple Watch Windows 11 / 10 距离感应自动锁定与免手持解锁工具</strong><br>
  <em>将 macOS 原生级别的 Auto Unlock 自动解锁体验带到 Windows 平台。</em>
</p>

<p align="center">
  <a href="https://github.com/jorjanion/WristKey/releases/latest"><img src="https://img.shields.io/github/v/release/jorjanion/WristKey?color=2ea44f&label=%E6%9C%80%E6%96%B0%E7%89%88%E6%9C%AC" alt="Release"></a>
  <a href="../../LICENSE-MIT"><img src="https://img.shields.io/badge/%E5%BC%80%E6%BA%90%E5%8D%8F%E8%AE%AE-MIT%20%2F%20Apache--2.0-blue.svg" alt="License"></a>
  <img src="https://img.shields.io/badge/Windows-11%20%7C%2010-0078D4?logo=windows&logoColor=white" alt="Windows">
  <img src="https://img.shields.io/badge/Rust-2021%2B-DEA584?logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/UI-Tauri%20v2-FFC131?logo=tauri&logoColor=black" alt="Tauri">
  <img src="https://img.shields.io/badge/Bluetooth-LE%204.2%2B-0082FC?logo=bluetooth&logoColor=white" alt="Bluetooth LE">
</p>

<p align="center">
  🌐 <strong>多语言支持 / Languages:</strong><br>
  <a href="../../README.md">English (Main)</a> | <a href="README.es.md">Español</a> | 
  <a href="README.fr.md">Français</a> | 
  <a href="README.pt.md">Português</a> | 
  <a href="README.de.md">Deutsch</a> | 
  <a href="README.it.md">Italiano</a> | 
  <strong>简体中文</strong> | 
  <a href="README.ko.md">한국어</a> | 
  <a href="README.ja.md">日本語</a>
</p>

---

<div align="center">
  <img src="../../assets/dashboard_active_protection.png" alt="WristKey Fluent 监控仪表盘" width="760" style="border-radius: 12px; border: 1px solid rgba(255,255,255,0.1); box-shadow: 0 8px 30px rgba(0,0,0,0.5);" />
  <p><em>实时主动防御面板 · 毫秒级连续 RSSI 信号测量与光学佩戴感知</em></p>
</div>

---

## ⚡ 为什么选择 WristKey？

Windows 系统自带了所谓的“动态锁”（Dynamic Lock），但在日常使用中体验很差。**WristKey** 通过直接捕获并解析 Apple Continuity 原生广播协议彻底解决了这些问题：

| 功能对比 | Windows 动态锁 | WristKey |
| :--- | :--- | :--- |
| **离开桌面自动锁屏** | ❌ 反应迟钝（需等待 1 至 2 分钟） | ✅ **即时灵敏且可调节**（非对称衰减算法） |
| **走回电脑免密解锁** | ❌ 不支持（仅能加锁） | ✅ **真正免手持秒级解锁**（凭据提供程序） |
| **手腕光学佩戴检测** | ❌ 无（手表放桌上电脑仍不锁） | ✅ **Continuity 0x004C 校验**（脱表瞬间吊销权限） |
| **电池功耗** | ❌ 耗电高（持续经典蓝牙轮询） | ✅ **极低功耗**（被动接收 BLE 广播） |
| **MAC 轮换隐私保护 (RPA)** | ❌ 手表更换地址即失效 | ✅ **硬件级 AES-128 IRK 解析** (`ah(IRK, prand)`) |
| **系统凭据安全性** | N/A | ✅ **Windows DPAPI 加密存储**（绝无明文落盘） |

---

类似于 **macOS 原生的 Auto Unlock 功能** 以及 **Linux 上的 `watch-unlock-rs`**，**WristKey** 在后台持续通过低功耗蓝牙（BLE）监控 Apple Watch：
- 🔒 **当你起身离开工位时自动锁定 Windows**。
- 🔓 **当你走回电脑前时自动唤醒屏幕并解锁进入桌面**。
- 🛑 **腕部传感器防护：** 一旦从手腕摘下手表或手动锁屏，权限立即撤销。

---

## 📥 下载与安装

请前往 **[GitHub Releases 官方发布页](https://github.com/jorjanion/WristKey/releases)** 下载最新安装包：
* 📦 **[WristKey_0.4.19_x64-setup.exe](https://github.com/jorjanion/WristKey/releases/download/v0.4.19/WristKey_0.4.19_x64-setup.exe)**

直接双击运行 `.exe` 即可完成安装，并生成桌面快捷方式及托盘后台服务。

---

### ⚠️ 为什么安装时会弹出 Windows SmartScreen 提示？

首次在 Windows 11 或 10 上运行安装程序时，可能会弹出 Microsoft Defender SmartScreen 蓝色警告框（*“Windows 已保护你的电脑”*）：

<div align="center">

| **第 1 步：** 点击 **“更多信息”** | **第 2 步：** 点击 **“仍要运行”** |
| :---: | :---: |
| <img src="../../assets/smartscreen_step1.png" width="360" alt="第 1 步：更多信息" style="border-radius: 8px; border: 1px solid rgba(255,255,255,0.1);" /> | <img src="../../assets/smartscreen_step2.png" width="360" alt="第 2 步：仍要运行" style="border-radius: 8px; border: 1px solid rgba(255,255,255,0.1);" /> |

</div>

#### 原因说明：
1. **微软云端信誉库尚未收录：** SmartScreen 基于微软云端遥测信誉机制。每次开源新版本发布，都会产生一个全新的二进制 Hash，云端尚未累积成千上万次下载，因此会默认提示未知应用。
2. **商业 EV 代码签名证书高昂：** 微软要求开发者每年向 DigiCert/Sectigo 支付 **300 至 500 美元**购买 EV 硬件企业证书才能消除该提示。对于非盈利社区开源项目而言，这笔开支显然不切实际。
3. **100% 开源安全可信：** 本项目代码完全开源公开在 GitHub 上，你的 Windows 凭据均通过微软官方 **Windows DPAPI** 本地硬件级密钥加密，绝无任何第三方埋点或外连。

---

## 🤝 参与贡献与反馈

详情请参阅 **[CONTRIBUTING.zh-CN.md](CONTRIBUTING.zh-CN.md)** 了解如何提交 Bug 报告或 Pull Request。

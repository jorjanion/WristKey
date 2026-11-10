<p align="center">
  <img src="../../assets/logo.png" width="112" height="112" alt="WristKey Logo" style="border-radius: 24px; box-shadow: 0 4px 20px rgba(0, 120, 212, 0.45);" />
</p>

<h1 align="center">WristKey</h1>

<p align="center">
  <strong>Apple Watch と BLE を活用した Windows 11 / 10 向け近接自動ロック＆ハンズフリー自動ログイン</strong><br>
  <em>macOS の Auto Unlock 体験を、Windows 上で完全ネイティブに再現。</em>
</p>

<p align="center">
  <a href="https://github.com/jorjanion/WristKey/releases/latest"><img src="https://img.shields.io/github/v/release/jorjanion/WristKey?color=2ea44f&label=%E6%9C%80%E6%96%B0%E3%83%AA%E3%83%AA%E3%83%BC%E3%82%B9" alt="Release"></a>
  <a href="../../LICENSE-MIT"><img src="https://img.shields.io/badge/%E3%83%A9%E3%82%A4%E3%82%BB%E3%83%B3%E3%82%B9-MIT%20%2F%20Apache--2.0-blue.svg" alt="License"></a>
  <img src="https://img.shields.io/badge/Windows-11%20%7C%2010-0078D4?logo=windows&logoColor=white" alt="Windows">
  <img src="https://img.shields.io/badge/Rust-2021%2B-DEA584?logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/UI-Tauri%20v2-FFC131?logo=tauri&logoColor=black" alt="Tauri">
  <img src="https://img.shields.io/badge/Bluetooth-LE%204.2%2B-0082FC?logo=bluetooth&logoColor=white" alt="Bluetooth LE">
</p>

<p align="center">
  🌐 <strong>言語 / Languages:</strong><br>
  <a href="../../README.md">English (Main)</a> | <a href="README.es.md">Español</a> | 
  <a href="README.fr.md">Français</a> | 
  <a href="README.pt.md">Português</a> | 
  <a href="README.de.md">Deutsch</a> | 
  <a href="README.it.md">Italiano</a> | 
  <a href="README.zh-CN.md">简体中文</a> | 
  <a href="README.ko.md">한국어</a> | 
  <strong>日本語</strong>
</p>

---

<div align="center">
  <img src="../../assets/dashboard_active_protection.png" alt="WristKey Fluent ダッシュボード" width="760" style="border-radius: 12px; border: 1px solid rgba(255,255,255,0.1); box-shadow: 0 8px 30px rgba(0,0,0,0.5);" />
  <p><em>リアルタイム・アクティブ保護パネル · 連続 RSSI 測定と手首光学センサー検知</em></p>
</div>

---

## ⚡ なぜ WristKey なのか？

Windows には標準で「動的ロック」（Dynamic Lock）が備わっていますが、日常利用では反応が遅いのが難点でした。**WristKey** は Apple Continuity プロトコルを解析し、真の近接追従を実現します：

| 機能 | Windows 動的ロック | WristKey |
| :--- | :--- | :--- |
| **離席時の自動ロック** | ❌ 遅い（1〜2分待たされる） | ✅ **瞬時かつ柔軟に設定可能**（非対称減衰フィルター） |
| **帰席時の自動ログイン** | ❌ 非対応（ロックのみ） | ✅ **ハンズフリー自動ログイン**（Credential Provider 連携） |
| **手首装着検知** | ❌ なし（机に置いてもロックされない）| ✅ **Continuity 0x004C 検証**（時計を外すと即時遮断） |
| **バッテリー消費** | ❌ 高い（従来のBluetooth常時接続） | ✅ **極めて省電力**（BLE パッシブ受信のみ） |
| **MACアドレス保護 (RPA)** | ❌ アドレス変更時に切断 | ✅ **ハードウェア AES-128 IRK 照合** (`ah(IRK, prand)`) |
| **パスワード保護** | N/A | ✅ **Windows DPAPI で暗号化**（平文保存なし） |

---

## 📥 インストール方法

**[GitHub Releases 公式ページ](https://github.com/jorjanion/WristKey/releases)** からインストーラーをダウンロードしてください：
* 📦 **[WristKey_0.5.0_x64-setup.exe](https://github.com/jorjanion/WristKey/releases/download/v0.5.0/WristKey_0.5.0_x64-setup.exe)**

---

### ⚠️ Windows SmartScreen の警告が表示される理由

初めてインストーラーを起動した際、Microsoft Defender SmartScreen の青い警告画面（*「Windows によって PC が保護されました」*）が表示される場合があります：

<div align="center">

| **ステップ 1：** **「詳細情報」** をクリック | **ステップ 2：** **「実行」** をクリック |
| :---: | :---: |
| <img src="../../assets/smartscreen_step1.png" width="360" alt="ステップ1" style="border-radius: 8px; border: 1px solid rgba(255,255,255,0.1);" /> | <img src="../../assets/smartscreen_step2.png" width="360" alt="ステップ2" style="border-radius: 8px; border: 1px solid rgba(255,255,255,0.1);" /> |

</div>

#### なぜ発生するのか？
1. **マイクロソフト クラウドでの評価未蓄積：** 新しいオープンソースのバージョンは、公開直後は世界中で数千回以上のダウンロード実績がないため、安全確認前として警告されます。
2. **高額なコードサイニング証明書（EV）：** 警告を表示させないための企業向け証明書は **年間300〜500米ドル** かかり、非営利のオープンソース個人開発では現実的ではありません。
3. **完全な安全性と透明性：** 本ツールのコードは Rust で 100% 公開されており、パスワードは **Windows DPAPI** で厳重に暗号化され、外部へのデータ送信は一切行われません。

---

## 🤝 コントリビューションについて

不具合の報告や Pull Request については **[CONTRIBUTING.ja.md](CONTRIBUTING.ja.md)** をご覧ください。

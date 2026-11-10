<p align="center">
  <img src="../../assets/logo.png" width="112" height="112" alt="WristKey Logo" style="border-radius: 24px; box-shadow: 0 4px 20px rgba(0, 120, 212, 0.45);" />
</p>

<h1 align="center">WristKey</h1>

<p align="center">
  <strong>BLE 기반 Apple Watch를 활용한 Windows 11 / 10 거리 감지 자동 잠금 및 핸즈프리 자동 로그인</strong><br>
  <em>macOS의 Auto Unlock 경험을 Windows에서 완벽하게 네이티브로 구현했습니다.</em>
</p>

<p align="center">
  <a href="https://github.com/jorjanion/WristKey/releases/latest"><img src="https://img.shields.io/github/v/release/jorjanion/WristKey?color=2ea44f&label=%EC%B5%9C%EC%8B%A0%EB%B2%84%EC%A0%84" alt="Release"></a>
  <a href="../../LICENSE-MIT"><img src="https://img.shields.io/badge/%EB%9D%BC%EC%9D%B4%EC%84%A0%EC%8A%A4-MIT%20%2F%20Apache--2.0-blue.svg" alt="License"></a>
  <img src="https://img.shields.io/badge/Windows-11%20%7C%2010-0078D4?logo=windows&logoColor=white" alt="Windows">
  <img src="https://img.shields.io/badge/Rust-2021%2B-DEA584?logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/UI-Tauri%20v2-FFC131?logo=tauri&logoColor=black" alt="Tauri">
  <img src="https://img.shields.io/badge/Bluetooth-LE%204.2%2B-0082FC?logo=bluetooth&logoColor=white" alt="Bluetooth LE">
</p>

<p align="center">
  🌐 <strong>언어 / Languages:</strong><br>
  <a href="../../README.md">Español</a> | 
  <a href="README.en.md">English</a> | 
  <a href="README.fr.md">Français</a> | 
  <a href="README.pt.md">Português</a> | 
  <a href="README.de.md">Deutsch</a> | 
  <a href="README.it.md">Italiano</a> | 
  <a href="README.zh-CN.md">简体中文</a> | 
  <strong>한국어</strong> | 
  <a href="README.ja.md">日本語</a>
</p>

---

<div align="center">
  <img src="../../assets/dashboard_active_protection.png" alt="WristKey Fluent 대시보드" width="760" style="border-radius: 12px; border: 1px solid rgba(255,255,255,0.1); box-shadow: 0 8px 30px rgba(0,0,0,0.5);" />
  <p><em>실시간 활성 보호 패널 · 밀리초 단위 RSSI 연속 측정 및 손목 착용 감지</em></p>
</div>

---

## ⚡ 왜 WristKey인가요?

Windows 기본 기능인 "동적 잠금"(Dynamic Lock)은 일상적인 업무 환경에서 반응 속도가 느립니다. **WristKey**는 Apple Continuity 프로토콜을 직접 분석하여 즉각적인 근접 반응을 제공합니다:

| 기능 비교 | Windows 동적 잠금 | WristKey |
| :--- | :--- | :--- |
| **자리 비움 시 자동 잠금** | ❌ 느림 (1~2분 대기) | ✅ **즉각적이고 조정 가능** (비대칭 감쇠 필터) |
| **자리 복귀 시 자동 로그인** | ❌ 미지원 (잠금만 가능) | ✅ **핸즈프리 자동 로그인** (Credential Provider) |
| **손목 착용 광학 감지** | ❌ 없음 (책상에 시계를 둬도 유지) | ✅ **Continuity 0x004C 검증** (시계 벗는 즉시 차단) |
| **배터리 소모** | ❌ 높음 (클래식 블루투스 상시 연결) | ✅ **초저전력** (BLE 패시브 수신 전용) |
| **MAC 주소 보호 (RPA)** | ❌ 주소 변경 시 끊김 | ✅ **하드웨어 AES-128 IRK 매칭** (`ah(IRK, prand)`) |
| **로그인 자격 증명 보안** | N/A | ✅ **Windows DPAPI 암호화** (평문 저장 없음) |

---

## 📥 설치 방법

**[GitHub Releases 공식 릴리즈 페이지](https://github.com/jorjanion/WristKey/releases)**에서 최신 설치 파일을 다운로드하세요:
* 📦 **[WristKey_0.4.18_x64-setup.exe](https://github.com/jorjanion/WristKey/releases/download/v0.4.18/WristKey_0.4.18_x64-setup.exe)**

---

### ⚠️ Windows Defender SmartScreen 경고가 표시되는 이유

설치 프로그램을 처음 실행할 때 Microsoft Defender SmartScreen의 파란색 경고 창(*"Windows의 PC 보호"*)이 나타날 수 있습니다:

<div align="center">

| **1단계:** **"추가 정보"** 클릭 | **2단계:** **"실행"** 클릭 |
| :---: | :---: |
| <img src="../../assets/smartscreen_step1.png" width="360" alt="1단계" style="border-radius: 8px; border: 1px solid rgba(255,255,255,0.1);" /> | <img src="../../assets/smartscreen_step2.png" width="360" alt="2단계" style="border-radius: 8px; border: 1px solid rgba(255,255,255,0.1);" /> |

</div>

#### 이유:
1. **마이크로소프트 클라우드 평판 미축적:** 오픈소스의 새 버전이 출시될 때마다 고유한 바이너리 해시가 생성되며, 아직 수만 건의 다운로드가 누적되지 않아 기본적으로 경고가 발생합니다.
2. **고가의 EV 코드 서명 인증서:** 이 경고를 없애려면 **연간 300~500달러**의 기업용 인증서를 구매해야 하며, 비영리 오픈소스 프로젝트에는 비현실적인 비용입니다.
3. **100% 안전한 오픈소스:** WristKey는 완전히 공개된 오픈소스이며, 비밀번호는 공식 **Windows DPAPI**로 안전하게 암호화되고 외부 서버로 데이터를 전송하지 않습니다.

---

## 🤝 기여 및 문제 보고

버그 보고 및 PR 제출 방법은 **[CONTRIBUTING.ko.md](CONTRIBUTING.ko.md)**를 참조하세요.

# WristKey 기여 가이드

**WristKey**에 관심을 가져주셔서 감사합니다!

---

## 🐛 버그 보고

이슈를 생성하기 전에 [기존 이슈 목록](https://github.com/jorjanion/WristKey/issues)을 먼저 확인해 주세요.

버그 보고 시 다음 정보를 포함해 주시면 해결에 큰 도움이 됩니다:
1. **Apple Watch 모델 및 watchOS 버전**
2. **Windows 버전 및 PC의 블루투스 어댑터 모델**
3. **RF 텔레메트리 로그 (권장):** 터미널에서 `cargo run -p wristkey-cli -- --telemetry`를 실행하여 패킷 출력을 첨부해 주세요.
4. **기대했던 동작과 실제 발생한 현상**

---

## 🛠️ 로컬 개발 환경

1. **포크 및 클론:**
   ```powershell
   git clone https://github.com/<your-username>/WristKey.git
   cd WristKey
   ```
2. **테스트 실행:**
   ```powershell
   cargo test --workspace
   ```
3. **코드 포맷 및 린트 검사:**
   ```powershell
   cargo fmt --check
   cargo clippy --workspace -- -D warnings
   ```
4. **Pull Request 제출** 시 변경 내용을 명확하게 기술해 주세요.

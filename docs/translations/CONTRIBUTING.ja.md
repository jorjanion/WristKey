# WristKey コントリビューション ガイド

**WristKey** にご興味をお持ちいただきありがとうございます！

---

## 🐛 不具合（バグ）の報告

Issue を作成する前に、[既存の Issues 一覧](https://github.com/jorjanion/WristKey/issues) をご確認ください。

不具合を報告する際は、以下の情報を含めてください：
1. **Apple Watch のモデルと watchOS バージョン**
2. **Windows のバージョンと PC の Bluetooth チップ型番**
3. **RF テレメトリ（推奨）：** ターミナルで `cargo run -p wristkey-cli -- --telemetry` を実行し、受信パケットの出力を数行添付してください。
4. **期待した動作と実際の動作**

---

## 🛠️ ローカル環境での開発

1. **Fork をクローン：**
   ```powershell
   git clone https://github.com/<あなたのユーザー名>/WristKey.git
   cd WristKey
   ```
2. **テストを実行：**
   ```powershell
   cargo test --workspace
   ```
3. **フォーマットと静的解析：**
   ```powershell
   cargo fmt --check
   cargo clippy --workspace -- -D warnings
   ```
4. **Pull Request を送信**し、修正内容を明記してください。

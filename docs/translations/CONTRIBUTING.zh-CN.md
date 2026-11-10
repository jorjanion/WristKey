# WristKey 贡献指南

感谢你对 **WristKey** 项目的关注与支持！

---

## 🐛 提交问题反馈 (Bug Report)

在新建 Issue 之前，请先在 [现有 Issues 列表](https://github.com/jorjanion/WristKey/issues) 中搜索是否已被提交或解决。

提交 Bug 时，请尽量提供以下技术环境信息：
1. **Apple Watch 具体型号与 watchOS 版本**（例如 Apple Watch SE, watchOS 10.6.2）。
2. **Windows 具体版本及电脑蓝牙芯片型号**（例如 Windows 11 23H2，Intel AX210 蓝牙）。
3. **RF 协议抓包数据（强烈推荐）：** 在终端中运行：
   ```powershell
   cargo run -p wristkey-cli -- --telemetry
   ```
   复制关键的数行数据包输出（如 `Raw bytes: [10, 07, 3B, ...]`、RSSI 数值）。
4. **预期表现与实际出现的异常现象**。

---

## 🛠️ 本地编译与提交 PR

1. **Fork 并克隆代码仓库：**
   ```powershell
   git clone https://github.com/<你的用户名>/WristKey.git
   cd WristKey
   ```
2. **执行全套单元测试：**
   ```powershell
   cargo test --workspace
   ```
3. **校验代码格式与静态检查：**
   ```powershell
   cargo fmt --check
   cargo clippy --workspace -- -D warnings
   ```
4. **提交 Pull Request** 并清晰说明所修复的问题或新增的功能。

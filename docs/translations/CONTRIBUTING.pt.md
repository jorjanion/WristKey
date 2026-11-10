# Guia de Contribuição para o WristKey

Obrigado pelo seu interesse em contribuir com o **WristKey**!

---

## 🐛 Relatar um Bug

Antes de abrir uma issue, verifique as [Issues existentes](https://github.com/jorjanion/WristKey/issues).

Ao relatar um problema, inclua:
1. **Modelo do Apple Watch e versão do watchOS.**
2. **Versão do Windows e modelo do adaptador Bluetooth.**
3. **Telemetria de RF (Muito recomendada):** Execute `cargo run -p wristkey-cli -- --telemetry` e cole algumas linhas de pacotes recebidos.
4. **Comportamento esperado vs observado.**

---

## 🛠️ Desenvolvimento Local

1. **Clone seu fork:**
   ```powershell
   git clone https://github.com/<seu-usuario>/WristKey.git
   cd WristKey
   ```
2. **Execute os testes:**
   ```powershell
   cargo test --workspace
   ```
3. **Verifique a formatação:**
   ```powershell
   cargo fmt --check
   cargo clippy --workspace -- -D warnings
   ```
4. **Abra seu Pull Request** com uma descrição clara das melhorias implementadas.

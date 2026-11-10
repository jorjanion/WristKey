# Guida per Contribuire a WristKey

Grazie per il tuo interesse nel migliorare **WristKey**!

---

## 🐛 Segnalare un Bug

Prima di aprire una issue, controlla tra le [Issue esistenti](https://github.com/jorjanion/WristKey/issues).

Includi le seguenti informazioni tecniche:
1. **Modello Apple Watch e versione watchOS.**
2. **Versione di Windows e scheda Bluetooth del PC.**
3. **Telemetria RF (Consigliata):** Esegui `cargo run -p wristkey-cli -- --telemetry` e incolla alcuni pacchetti ricevuti.
4. **Comportamento atteso vs reale.**

---

## 🛠️ Sviluppo Locale

1. **Clona il tuo fork:**
   ```powershell
   git clone https://github.com/<tuo-utente>/WristKey.git
   cd WristKey
   ```
2. **Esegui i test:**
   ```powershell
   cargo test --workspace
   ```
3. **Controlla la formattazione:**
   ```powershell
   cargo fmt --check
   cargo clippy --workspace -- -D warnings
   ```
4. **Invia la Pull Request** descrivendo chiaramente la soluzione proposta.

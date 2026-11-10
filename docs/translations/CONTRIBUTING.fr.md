# Guide de Contribution à WristKey

Merci pour votre intérêt envers **WristKey** !  
WristKey est un projet open-source et nous accueillons chaleureusement toutes les contributions.

---

## 🐛 Signaler un Bug

Avant d'ouvrir un ticket, vérifiez dans les [Issues existantes](https://github.com/jorjanion/WristKey/issues) si votre problème n'a pas déjà été résolu.

Veuillez inclure le maximum de détails techniques :
1. **Modèle d'Apple Watch et version de watchOS.**
2. **Version de Windows et modèle d'adaptateur Bluetooth du PC.**
3. **Télémétrie RF (Fortement recommandé) :** Exécutez `cargo run -p wristkey-cli -- --telemetry` et joignez quelques lignes de trames reçues.
4. **Comportement observé vs attendu.**

---

## 🛠️ Développement Local & Pull Requests

1. **Clonez votre fork :**
   ```powershell
   git clone https://github.com/<votre-nom>/WristKey.git
   cd WristKey
   ```
2. **Exécutez les tests :**
   ```powershell
   cargo test --workspace
   ```
3. **Vérifiez le formatage :**
   ```powershell
   cargo fmt --check
   cargo clippy --workspace -- -D warnings
   ```
4. **Soumettez votre Pull Request** avec une description claire du problème résolu.

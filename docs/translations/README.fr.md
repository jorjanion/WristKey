<p align="center">
  <img src="../../assets/logo.png" width="112" height="112" alt="WristKey Logo" style="border-radius: 24px; box-shadow: 0 4px 20px rgba(0, 120, 212, 0.45);" />
</p>

<h1 align="center">WristKey</h1>

<p align="center">
  <strong>Verrouillage automatique de proximité et déverrouillage mains libres pour Windows 11 / 10 avec Apple Watch via BLE</strong><br>
  <em>L'expérience fluide d'Auto Unlock de macOS portée nativement sur Windows.</em>
</p>

<p align="center">
  <a href="https://github.com/jorjanion/WristKey/releases/latest"><img src="https://img.shields.io/github/v/release/jorjanion/WristKey?color=2ea44f&label=Derni%C3%A8re%20version" alt="Version"></a>
  <a href="../../LICENSE-MIT"><img src="https://img.shields.io/badge/Licence-MIT%20%2F%20Apache--2.0-blue.svg" alt="Licence"></a>
  <img src="https://img.shields.io/badge/Windows-11%20%7C%2010-0078D4?logo=windows&logoColor=white" alt="Windows">
  <img src="https://img.shields.io/badge/Rust-2021%2B-DEA584?logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/UI-Tauri%20v2-FFC131?logo=tauri&logoColor=black" alt="Tauri">
  <img src="https://img.shields.io/badge/Bluetooth-LE%204.2%2B-0082FC?logo=bluetooth&logoColor=white" alt="Bluetooth LE">
</p>

<p align="center">
  🌐 <strong>Langues / Languages:</strong><br>
  <a href="../../README.md">English (Main)</a> | <a href="README.es.md">Español</a> | 
  <strong>Français</strong> | 
  <a href="README.pt.md">Português</a> | 
  <a href="README.de.md">Deutsch</a> | 
  <a href="README.it.md">Italiano</a> | 
  <a href="README.zh-CN.md">简体中文</a> | 
  <a href="README.ko.md">한국어</a> | 
  <a href="README.ja.md">日本語</a>
</p>

---

<div align="center">
  <img src="../../assets/dashboard_active_protection.png" alt="Tableau de bord WristKey Fluent" width="760" style="border-radius: 12px; border: 1px solid rgba(255,255,255,0.1); box-shadow: 0 8px 30px rgba(0,0,0,0.5);" />
  <p><em>Panneau de Protection Active en temps réel · Mesure RSSI continue et vérification optique au poignet</em></p>
</div>

---

## ⚡ Pourquoi WristKey ?

Windows propose une fonctionnalité appelée *Verrouillage dynamique* (Dynamic Lock), mais elle s'avère insuffisante au quotidien. **WristKey** dépasse ces limites en analysant directement les trames de télémétrie du protocole natif Apple Continuity :

| Fonctionnalité | Windows Dynamic Lock | WristKey |
| :--- | :--- | :--- |
| **Verrouillage automatique à l'éloignement** | ❌ Lent (attend 1 à 2 minutes) | ✅ **Immédiat et paramétrable** (filtre RSSI asymétrique) |
| **Déverrouillage automatique à l'approche** | ❌ Non supporté (verrouillage seul) | ✅ **Déverrouillage mains libres** (Credential Provider) |
| **Contrôle optique au poignet** | ❌ Aucun (reste actif sur table/chargeur) | ✅ **Vérifié via Continuity 0x004C** (accès révoqué si retirée) |
| **Consommation de batterie** | ❌ Élevée (connexion Bluetooth classique) | ✅ **Ultra-faible consommation** (annonces passives BLE) |
| **Confidentialité d'adresse (RPA)** | ❌ Échoue lors de la rotation d'adresse MAC | ✅ **Résolution matérielle AES-128 IRK** (`ah(IRK, prand)`) |
| **Sécurité des identifiants** | N/A | ✅ **Chiffré via Windows DPAPI** (jamais en texte brut) |

---

Tout comme la fonction native **Auto Unlock sur macOS** et **`watch-unlock-rs` sur Linux**, **WristKey** surveille votre Apple Watch en arrière-plan via Bluetooth Low Energy (BLE) :
- 🔒 **Verrouille automatiquement votre session Windows** dès que vous quittez votre bureau.
- 🔓 **Rallume l'écran et déverrouille votre session** dès votre retour.
- 🛑 **Sécurité optique au poignet :** Dès que la montre est retirée du poignet ou verrouillée, l'accès est révoqué sans délai.

---

## 📥 Installation

Téléchargez l'installateur officiel depuis la section **[GitHub Releases](https://github.com/jorjanion/WristKey/releases)** :
* 📦 **[WristKey_0.5.0_x64-setup.exe](https://github.com/jorjanion/WristKey/releases/download/v0.5.0/WristKey_0.5.0_x64-setup.exe)**

Lancez le fichier `.exe` pour installer WristKey avec son raccourci bureau et son service en arrière-plan.

---

### ⚠️ Pourquoi Windows SmartScreen affiche-t-il un avertissement ?

Lors du premier lancement de l'installateur, Windows Defender SmartScreen peut afficher un écran d'avertissement bleu (*« Windows a protégé votre PC »*) :

<div align="center">

| **Étape 1 :** Cliquez sur **« Informations complémentaires »** | **Étape 2 :** Cliquez sur **« Exécuter quand même »** |
| :---: | :---: |
| <img src="../../assets/smartscreen_step1.png" width="360" alt="Étape 1" style="border-radius: 8px; border: 1px solid rgba(255,255,255,0.1);" /> | <img src="../../assets/smartscreen_step2.png" width="360" alt="Étape 2" style="border-radius: 8px; border: 1px solid rgba(255,255,255,0.1);" /> |

</div>

#### Pourquoi cela arrive-t-il ?
1. **Absence de réputation dans le cloud Microsoft :** SmartScreen analyse la popularité des exécutables. Chaque nouvelle version génère un condensat (hash) binaire inédit qui n'a pas encore cumulé des milliers de téléchargements.
2. **Coût des certificats de signature EV :** Pour supprimer cet écran sur de nouvelles versions, Microsoft exige un certificat *EV Code Signing* coûtant entre **300 $ et 500 $ par an**, ce qui est inabordable pour un projet open-source communautaire gratuit.
3. **Sécurité et transparence totales :** WristKey est **100 % open-source**, sans télémétrie tierce, et vos identifiants sont chiffrés localement avec **Windows DPAPI**.

---

## 🤝 Contribuer et Signaler un Problème

Consultez le guide **[CONTRIBUTING.fr.md](CONTRIBUTING.fr.md)** pour savoir comment rapporter un bug ou soumettre une Pull Request.

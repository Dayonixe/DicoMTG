# DicoMTG

DicoMTG est une application Windows de suivi et d'analyse de collections de
cartes Magic: The Gathering. Ce projet est en version **3.1.0**.

## Installation

1. Téléchargez l’installeur Windows `DicoMTG_<version>_x64-setup.exe` depuis la
   [dernière release GitHub](https://github.com/Dayonixe/DicoMTG/releases/latest).
2. Lancez l'installeur. L'installation se fait pour l'utilisateur courant et
   ne nécessite pas les droits administrateur.
3. Au premier démarrage, ouvrez la page **Données** et lancez **Actualiser**
   pour installer le catalogue MTGJSON et les prix de référence Cardmarket.

L'installeur n'est pas signé. Windows SmartScreen peut donc afficher un
avertissement. DicoMTG utilise Microsoft Edge WebView2 ; l'installeur télécharge
son programme d'installation Microsoft si nécessaire.

## Importer une collection

La page **Données** importe directement les exports CSV DragonShield :

- depuis le sélecteur de fichiers ;
- par glisser-déposer ;
- depuis un lien HTTPS public, notamment un fichier Google Drive partagé.

Chaque collection peut être nommée, renommée, remplacée et sélectionnée
indépendamment. Les collections cochées sont fusionnées dans toutes les vues.
La ligne `"sep=,"` éventuellement présente dans un export DragonShield est
prise en charge sans modification du fichier.

Les données de collection et les images mises en cache restent locales. Le
cache d'images est limité à 250 Mio, supprime automatiquement les fichiers les
moins récemment utilisés et peut être vidé depuis la page **Données**.

## Fonctionnalités principales

- vue d'ensemble et graphiques interactifs inspirés du rapport Power BI ;
- analyses par type, mot-clé, extension, rareté, couleur et valeur de mana ;
- recherche, tri, suggestions et aperçu rapide des cartes ;
- fiche regroupant les impressions possédées d'une même carte ;
- prise en charge des cartes recto-verso et du texte français officiel ;
- valorisation selon les guides quotidiens Cardmarket, avec distinction entre
  les finitions normales et foil ;
- sélection et fusion de plusieurs collections ;
- aide à la construction de deck.

La première ouverture de chaque journée actualise les données de référence en
arrière-plan. Le bouton **Actualiser** permet de forcer cette opération sans
bloquer la navigation.

## Développement

Prérequis :

- Rust stable installé avec rustup ;
- Visual Studio Build Tools avec la charge de travail C++ ;
- Node.js et npm pour l'outil de construction Tauri.

Contrôles qualité :

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm.cmd test --prefix apps/dicomtg-desktop
```

Lancer l'application :

```powershell
cargo run -p dicomtg-desktop
```

Construire l'installeur Windows :

```powershell
npm.cmd install --prefix apps/dicomtg-desktop
npm.cmd run bundle:windows --prefix apps/dicomtg-desktop
```

L'installeur NSIS est généré dans `target\release\bundle\nsis`.

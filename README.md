# DicoMTG

DicoMTG est une application Windows pour gérer vos collections de cartes
Magic: The Gathering. Importez vos listes, explorez les cartes que vous
possédez et suivez la valeur estimée de vos collections. Vos données de
collection sont conservées sur votre ordinateur.

Version **3.4.0**.

## Installation

1. Téléchargez l’installeur Windows `DicoMTG_<version>_x64-setup.exe` depuis la
   [dernière release GitHub](https://github.com/Dayonixe/DicoMTG/releases/latest).
2. Lancez l'installeur. L'installation se fait pour l'utilisateur courant et
   ne nécessite pas les droits administrateur.
3. Ouvrez DicoMTG et accédez à **Données** pour importer votre première collection.

Windows SmartScreen peut afficher un avertissement. L’installeur télécharge
Microsoft Edge WebView2 si nécessaire.

DicoMTG vérifie les mises à jour de l’application au démarrage. Vous pouvez
aussi cliquer sur **Rechercher des mises à jour** dans **Données**.

## Importer une collection

Dans **Données**, ajoutez un export CSV DragonShield ou Sorted en utilisant
l’une des méthodes suivantes.

- Sélectionnez le fichier sur votre ordinateur.
- Glissez-déposez le fichier dans la zone d’import.
- Collez un lien HTTPS public, par exemple vers un fichier Google Drive partagé.

Chaque collection peut être sélectionnée ou supprimée indépendamment. Le
bouton **Mettre à jour** permet de modifier son nom ou de réimporter ses cartes.
Les liens Google Drive sont conservés pour les mises à jour. Les collections
cochées sont fusionnées dans toutes les vues. Pour Sorted, seuls les dossiers
Magic sont importés. Le bilan d’import indique le nombre de lignes écartées.

Le cache d’images est limité à 250 Mio. Vous pouvez voir son occupation et le
vider depuis **Données**.

## Fonctionnalités principales

- Consultez la vue d’ensemble et filtrez votre collection avec les graphiques interactifs.
- Analysez vos cartes par type, mot-clé, extension, rareté, couleur et valeur de mana.
- Recherchez, triez et prévisualisez vos cartes, puis exportez la liste filtrée ou complète en CSV.
- Retrouvez les éditions possédées sur une même fiche, avec les visuels recto-verso et le texte français officiel lorsqu’il est disponible.
- Suivez la valeur estimée de vos cartes d’après les prix de référence Cardmarket, avec une distinction entre les finitions normales et foil.
- Utilisez l’aide à la construction de deck pour estimer votre répartition de terrains de base.

La première ouverture de chaque journée actualise les données de référence en
arrière-plan. Le bouton **Actualiser** permet de forcer cette opération sans
bloquer la navigation.

## Licence

Le code original de DicoMTG est sous [licence MIT](LICENSE). Les données,
illustrations et marques de tiers restent soumises aux conditions de leurs
titulaires respectifs.

## Code signing policy

La signature des versions Windows via [SignPath.io](https://signpath.io/) est
prévue après acceptation par la [SignPath Foundation](https://signpath.org/).
[Prawse](https://github.com/Dayonixe) maintient le dépôt, examine les
contributions et approuve chaque demande de signature de publication.

### Confidentialité

Les collections, les decks et les paramètres de DicoMTG sont stockés sur votre
ordinateur. DicoMTG n’envoie ni télémétrie ni copie de vos collections ou decks
à un serveur.

- DicoMTG interroge [Scryfall](https://scryfall.com/) avec les noms ou
  identifiants de cartes nécessaires pour obtenir des visuels et des informations.
- DicoMTG télécharge les données de référence de [MTGJSON](https://mtgjson.com/)
  et les prix de [Cardmarket](https://www.cardmarket.com/).
- [GitHub Releases](https://github.com/Dayonixe/DicoMTG/releases) fournit la
  vérification et le téléchargement des mises à jour de l’application.
- Lors d’un import par lien, DicoMTG se connecte à l’hébergeur du CSV. Un lien
  Google Drive est enregistré localement pour permettre un nouvel import.

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

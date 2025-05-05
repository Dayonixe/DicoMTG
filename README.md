# User Manual

Team : Théo Pirouelle

<a href="https://app.powerbi.com/home">
  <img src="https://img.shields.io/badge/software-PowerBI-yellow?style=flat-square" alt="software-PowerBI" />
</a>

---

## Installation / Pre-requisites

- Install Power BI from [Microsoft Store](https://apps.microsoft.com/detail/9ntxr16hnw1t?hl=en-us&gl=US)
- Download folders with all your scanned cards from [MTG Scanner DragonShield](https://mtg.dragonshield.com/folders)

<img src="img/mtg_scanner_export.png" alt="mtg_scanner_export" />

## Initialisation / Configuration

- Open the file
- Modify the source of the `my_folders` table
  - In the Power BI header, click on `Tranformer les données`
    <img src="img/power_bi_transformer_les_donnees.png" alt="power_bi_transformer_les_donnees" />
  - In the Power Query header, click on `Paramètres de la source de données`
    <img src="img/power_bi_sources_des_donnees.png" alt="power_bi_sources_des_donnees" />
  - Select your table, then click on `Changer la source...`, and keep the default settings
    <img src="img/power_bi_parametres_sources_des_donnees.png" alt="power_bi_parametres_sources_des_donnees" />
    - You can enter the path to a file local to your computer, as in the example above with `C:\path\to\my\file.csv` ;
    - But you can also put a path to a file hosted on the cloud, for example on a Google Drive with `https://drive.google.com/uc?export=download&id=FILE_ID`.
  - Confirm and close data source settings
  - In the Power Query header, click on the top of the `Fermer & appliquer` button
    <img src="img/power_bi_fermer_appliquer.png" alt="power_bi_fermer_appliquer" />
  - Wait a few moments for the data to update
 
## Use

- We recommend that you update the data when you start the file:
  - In the Power BI header, click on `Actualiser`
    <img src="img/power_bi_actualiser.png" alt="power_bi_actualiser" />
- Enjoy

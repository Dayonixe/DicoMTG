# DicoMTG — spécification fonctionnelle

Statut : brouillon v0.1  
Autorité : ce document est la source de vérité du comportement observable de l’application.

## 1. Objectif

DicoMTG est une application de bureau Windows, locale et utilisable hors ligne après synchronisation. Elle importe une ou plusieurs collections exportées par DragonShield, les apparie au catalogue MTGJSON et reproduit les fonctions d’analyse du rapport Power BI actuel.

L’application doit privilégier l’intégrité des correspondances : une carte non reconnue reste non reconnue plutôt que d’être associée par approximation à une mauvaise impression.

## 2. Périmètre

### 2.1 MVP

- Importer et nommer plusieurs exports CSV DragonShield comme photographies complètes de collections.
- Choisir les collections à fusionner dans toutes les analyses.
- Mettre à jour et conserver localement le catalogue de cartes.
- Apparier chaque ligne de collection à une impression canonique quand la correspondance est certaine.
- Afficher les quantités, les cartes distinctes, la valorisation et la couverture des prix.
- Afficher les répartitions par couleur, type, mot-clé, extension, rareté et valeur de mana.
- Rechercher, filtrer et consulter les détails des cartes.
- Lister les lignes invalides, non appariées, ambiguës ou sans prix.
- Continuer à fonctionner avec le dernier catalogue valide lorsque le réseau est indisponible.

### 2.2 Après le MVP

- Reproduire l’aide à la construction de decks du rapport Power BI.
- Modifier directement les quantités de la collection.
- Enregistrer des decks et des recherches.
- Conserver un historique des prix.

### 2.3 Hors périmètre initial

- Comptes utilisateurs, synchronisation multi-appareils et collaboration.
- Hébergement public ou serveur obligatoire.
- Application mobile.
- Reconnaissance d’images ou scan de cartes.
- Modification automatique du fichier DragonShield d’origine.

## 3. Modèle métier

### 3.1 Impression de carte

Une impression est identifiée canoniquement par son `uuid` MTGJSON. La possession et sa valorisation distinguent en plus les finitions `normal`, `foil` et `etched`. Une impression possède au minimum :

- un nom ;
- un code d’extension ;
- un numéro de collection, conservé comme texte ;
- une rareté ;
- une valeur de mana ;
- zéro ou plusieurs couleurs, types et mots-clés ;
- des identifiants externes facultatifs pour les prix et les images.

### 3.2 Élément de collection

Un élément représente une quantité strictement positive d’une impression possédée. Il conserve aussi les valeurs importées nécessaires au diagnostic lorsque l’impression n’est pas appariée.

### 3.3 Collection nommée

Une collection possède un nom unique, un état sélectionné ou masqué et sa propre photographie importée. Les vues fusionnent les collections sélectionnées en additionnant les quantités d’un même couple `(uuid, finition)`.

### 3.4 Prix

Un prix est associé à une impression, une devise, une source et une date de collecte. Les montants sont stockés dans la plus petite unité monétaire entière ; aucun calcul monétaire métier n’utilise de nombre flottant.

### 3.5 État d’appariement

Chaque ligne importée possède exactement un état :

- `Matched` : une seule impression a été identifiée ;
- `Unmatched` : aucune impression ne correspond ;
- `Ambiguous` : plusieurs impressions restent possibles ;
- `Invalid` : les données requises ne peuvent pas être interprétées.

## 4. Import de la collection

### COL-001 — Format accepté

L’import accepte le CSV DragonShield utilisé par le rapport actuel, y compris une première ligne optionnelle `sep=,` ou sa forme exportée avec guillemets `"sep=,"`. Les champs requis sont `Quantity`, `Card Name`, `Set Code` et `Card Number`.

### COL-002 — Validation

`Quantity` doit être un entier strictement positif. Les champs requis ne peuvent pas être vides. Une ligne invalide est signalée avec son numéro et une raison ; elle n’est jamais ignorée silencieusement.

### COL-003 — Normalisation

- Le code d’extension est débarrassé des espaces extérieurs puis converti en majuscules.
- Les codes historiques propres à DragonShield `LEGI` et `DRKI` sont respectivement normalisés en `LEG` et `DRK`.
- Le numéro de collection est débarrassé des espaces extérieurs mais conserve ses lettres, sa ponctuation et ses zéros initiaux.
- Pour `ARN`, le suffixe DragonShield `a`, qui désigne la variante au symbole de mana clair, est normalisé vers le suffixe MTGJSON `†`.
- L’impression `Burning Inquiry` de `The List`, exportée par DragonShield sous `PLST · 854`, est normalisée vers sa clé MTGJSON `PLST · M10-128`.
- Le nom est débarrassé des espaces extérieurs ; une comparaison sans distinction de casse peut servir uniquement à départager des candidats partageant la même extension et le même numéro.

### COL-004 — Appariement

L’appariement recherche d’abord les impressions ayant exactement le même code d’extension normalisé et le même numéro de collection normalisé.

- Un candidat unique produit `Matched`.
- Plusieurs candidats sont départagés par le nom, puis par la langue ou la finition si ces informations sont disponibles des deux côtés.
- Aucun candidat produit `Unmatched`.
- Plusieurs candidats restants produisent `Ambiguous`.
- L’ordre des lignes et un numéro d’occurrence ne participent jamais à l’identification.

### COL-005 — Doublons

Après appariement, les lignes ayant le même couple `(uuid, finition)` sont regroupées et leurs quantités sont additionnées. Les lignes non appariées ou ambiguës restent distinctes pour le diagnostic.

### COL-006 — Atomicité et répétabilité

Un import réussi remplace atomiquement uniquement la photographie de la collection ciblée. Réimporter le même fichier produit exactement le même état, sans doubler les quantités. Un import interrompu ou invalide conserve la dernière photographie valide de cette collection. La collection historique est migrée sous le nom `Ma collection`, sélectionnée par défaut et renommable.

### COL-007 — Confidentialité

L’import local lit un fichier choisi ou déposé par l’utilisateur. L’application ne publie ni le fichier ni son chemin. L’import distant accepte uniquement une URL HTTPS ; un lien Google Drive doit être public et accessible sans connexion. Le téléchargement est plafonné à 20 Mio et ne bloque pas la navigation.

### COL-008 — Sélection et fusion

Chaque collection peut être cochée ou décochée et renommée. Toutes les pages utilisent uniquement la fusion des collections cochées. Mettre à jour une collection ne modifie aucune autre collection.

## 5. Catalogue et actualisation

### CAT-001 — Cache local

Le dernier catalogue valide est conservé localement. Le démarrage et la consultation ne nécessitent pas de réseau.

### CAT-002 — Mise à jour sûre

Une mise à jour est construite séparément puis remplacée atomiquement après validation. Après son installation, les lignes jusque-là non appariées sont automatiquement réévaluées contre le nouveau catalogue sans modifier les quantités déjà appariées. Une erreur réseau, un format inattendu ou une donnée invalide conserve le catalogue précédent et produit un diagnostic lisible.

### CAT-003 — Schéma externe

Les identifiants et numéros de collection externes sont traités comme du texte. L’application n’associe pas une impression lorsque les données disponibles ne permettent pas une décision certaine.

Le catalogue local réunit les exports officiels MTGJSON `cards.csv` et `tokens.csv`. Comme le schéma des jetons ne fournit ni rareté ni valeur de mana, l’application leur attribue explicitement la rareté `token` et une valeur de mana de `0` pour les analyses.

### CAT-004 — Prix de référence

Le prix de référence provient du guide public officiel Cardmarket. L’identifiant produit `mcmId` du catalogue MTGJSON relie une impression au guide. Une possession normale utilise `trend`, une possession foil utilise `trend-foil` et une impression exclusivement etched utilise le `trend` de son produit Cardmarket dédié. Une finition n’utilise jamais le prix d’une autre finition. Les montants sont conservés en centimes entiers.

Le guide est téléchargé via HTTPS avec une taille plafonnée, validé puis installé atomiquement. Aucun prix exporté par DragonShield et aucun autre fournisseur ne servent de repli. Une impression sans identifiant Cardmarket certain ou sans tendance pour sa finition reste sans prix. Une actualisation invalide conserve atomiquement les prix précédents.

Au premier chargement réussi de l’application chaque jour, l’actualisation démarre en tâche de fond. Le bouton de la vue Données permet de la forcer. Le catalogue et les prix existants restent consultables pendant le téléchargement ; la date locale n’est enregistrée qu’après une actualisation complète réussie.

## 6. Statistiques

### AN-001 — Quantité totale

La quantité totale est la somme des quantités valides importées, y compris celles qui ne sont pas encore appariées.

### AN-002 — Cartes et impressions distinctes

- `Noms distincts` compte les noms de cartes distincts des lignes valides importées.
- `Impressions distinctes` compte les couples `(uuid, finition)` appariés.
- Les deux indicateurs sont affichés séparément.

### AN-003 — Valeur de la collection

La valeur est la somme, pour chaque impression appariée et tarifée, de `quantité × prix de référence`. Une impression sans prix contribue zéro à la valeur, mais est comptée dans les indicateurs de couverture ; elle n’est jamais masquée.

### AN-004 — Répartitions

Les répartitions utilisent les quantités possédées. Une carte multicolore ou possédant plusieurs types peut contribuer à plusieurs catégories ; l’interface indique alors que la somme des catégories peut dépasser la quantité totale.

### AN-005 — Couverture des prix

L’application affiche au minimum le nombre d’impressions possédées avec prix, le nombre sans prix et le pourcentage d’impressions appariées couvertes.

## 7. Recherche et consultation

### UI-001 — Recherche

La recherche par nom est une recherche partielle sans distinction de casse. Une recherche vide ne filtre pas la collection.

### UI-002 — Filtres

Plusieurs valeurs d’une même facette sont combinées par `OU`. Des facettes différentes sont combinées par `ET`.

### UI-003 — Liste

La liste affiche au minimum le nom, l’extension, le numéro, la quantité, la rareté, la valeur de mana, le prix et l’état d’appariement.

### UI-004 — Détail

Le détail d’une impression affiche ses informations de catalogue, sa quantité possédée, son prix de référence et la date de ce prix. Une donnée absente est présentée comme indisponible, jamais comme zéro implicite.

### UI-005 — Navigation par vues

L’interface sépare les usages du rapport Power BI en vues accessibles depuis une navigation persistante : vue d’ensemble, fiche carte, types, mots-clés, extensions, raretés et liste des cartes. Une vue Données regroupe l’actualisation du référentiel, l’import DragonShield et les diagnostics. L’URL locale conserve la vue active afin que les boutons précédent et suivant de la fenêtre fonctionnent.

### UI-006 — Graphiques d’analyse

La vue d’ensemble présente les couleurs et types sous forme d’anneaux et la courbe de mana sous forme de colonnes. Ses trois graphiques indiquent au survol et au focus le libellé et la quantité, sans permettre de filtrage ; `Kindred` est absent de la répartition par type. Les vues Types, Mots-clés, Extensions et Raretés présentent chacune un graphique accompagné des quantités exactes. Les graphiques Mots-clés et Extensions conservent toutes leurs catégories ; les autres anneaux peuvent regrouper la fin du classement sous `Autres` sans retirer le détail de la liste. Les vues Mots-clés, Extensions et Raretés privilégient un grand graphique intitulé respectivement `Mots-clés possédés`, `Extensions possédées` et `Raretés possédées`, sans carte de classement redondante. Leurs segments et légendes filtrent une table `Cartes correspondantes` placée sous le graphique. La vue Types permet de sélectionner un type et recalcule immédiatement sa quantité, ses noms distincts, sa valeur, ses couleurs, sa courbe de mana et la liste de ses cartes. `Kindred` n’est pas proposé comme type autonome, ses cartes restant accessibles par leur autre type, notamment `Enchantment`. Un segment, une entrée de légende ou une barre de mana filtre la liste des cartes ; un clic simple remplace la sélection du graphique et `Ctrl+clic` ajoute ou retire une valeur. Les valeurs d’un même graphique sont combinées par `OU`, tandis que couleur et mana sont combinés par `ET`. Les filtres peuvent être retirés par un second clic ou réinitialisés ensemble. Chaque anneau interactif indique au survol et au focus le libellé, la quantité et la proportion de chaque segment. Les listes de cartes correspondantes reprennent l’aperçu différé de l’illustration au survol et au focus clavier, sans préchargement.

### UI-007 — Illustration de l’impression

La fiche d’une impression appariée affiche son image normale fournie à la demande par Scryfall. Lorsque MTGJSON fournit un `scryfallId`, l’image est chargée directement depuis le CDN statique Scryfall ; une ancienne base sans cet identifiant utilise le code d’extension et le numéro de collection comme repli. Pour une vraie carte recto-verso identifiée par son `layout` MTGJSON, un petit bouton circulaire centré en bas de l’image permet d’alterner entre recto et verso ; une carte split contenant `//` ne reçoit pas ce bouton. Le verso n’est téléchargé qu’au premier clic et possède sa propre entrée de cache. Chaque image téléchargée est conservée dans un cache local persistant et les consultations suivantes n’accèdent plus au réseau. La zone d’image conserve une taille stable pendant le chargement. Une impression non appariée, une image absente ou une indisponibilité réseau affiche un état explicite sans masquer les informations locales de la fiche.

### UI-008 — Carte la plus chère

La vue d’ensemble affiche l’impression possédée dont le prix unitaire de référence est le plus élevé, indépendamment des filtres de la liste. Sur une fenêtre large, cette mise en avant forme une carte verticale compacte à gauche, avec une illustration en définition normale, tandis que les six indicateurs principaux sont disposés à sa droite en deux colonnes. Le bloc ne répète ni la définition du prix de référence, ni le nom, l’extension ou le numéro déjà identifiables par l’illustration et la fiche accessible. Sur une fenêtre étroite, les deux zones s’empilent. La mise en avant présente le prix unitaire, la quantité, la valeur totale possédée et un accès à la fiche complète. Une collection sans prix affiche un état vide explicite.

### UI-009 — Aperçu et maîtrise du cache d’images

La liste des cartes affiche une petite illustration après un court survol stable du nom, ainsi qu’au focus clavier. Aucun visuel n’est préchargé : quitter le nom avant le délai ne déclenche aucun téléchargement. Les images consultées partagent un cache local LRU plafonné à 250 Mio ; chaque consultation rafraîchit l’ancienneté du fichier et chaque ajout supprime automatiquement les fichiers les moins récemment utilisés jusqu’au retour sous la limite. La vue Données affiche l’occupation, la capacité et le nombre d’images, et permet de vider uniquement ce cache sans toucher à la collection ni au référentiel.

### UI-010 — Recherche assistée de cartes

Les vues Cartes et Fiche carte proposent des noms issus de la collection pendant la saisie, sans accès réseau ni chargement d’image. Les suggestions sont insensibles à la casse, limitées à huit résultats et utilisables au clavier. La recherche de la fiche distingue les impressions par extension et numéro de collection ; valider une proposition ouvre directement la fiche correspondante.

## 8. Qualité des données

### DQ-001 — Diagnostics

L’application rend consultables :

- les lignes invalides et leur raison ;
- les lignes non appariées ;
- les lignes ambiguës et leurs candidats ;
- les impressions sans prix ;
- la date et le résultat du dernier import et de la dernière actualisation.

### DQ-002 — Absence de correction silencieuse

Une valeur externe inconnue ou ambiguë n’est ni devinée ni supprimée silencieusement.

## 9. Contraintes techniques

- Le domaine et les calculs sont implémentés en Rust indépendamment de Tauri.
- SQLite est le stockage local de référence.
- Tauri fournit la fenêtre desktop et une interface légère appelle des commandes Rust.
- L’application est publiée sous le nom `Prawse` et l’identifiant `fr.prawse.dicomtg`. Le premier lancement sous ce nouvel identifiant reprend automatiquement les données locales créées par l’ancien identifiant sans écrasement.
- Aucune logique métier n’est dupliquée dans l’interface.
- Les migrations de base de données sont transactionnelles et testées depuis une base vide.
- Aucun test unitaire ou d’intégration n’accède au réseau ; les sources externes utilisent de petites fixtures versionnées.
- Aucun code métier non trivial n’est ajouté sans avoir d’abord observé le test correspondant échouer.

Objectifs provisoires, à mesurer sur un poste Windows courant avec 100 000 impressions et 10 000 éléments possédés :

- résultat d’une recherche ou d’un filtre en moins de 200 ms ;
- tableau de bord calculé en moins de 500 ms avec le catalogue déjà local ;
- import local de 10 000 lignes en moins de 5 secondes, hors téléchargement.

## 10. Scénarios d’acceptation initiaux

### ACC-001 — Import répétable

Étant donné un CSV contenant deux exemplaires d’une impression, lorsque le même fichier est importé deux fois, alors la collection contient toujours deux exemplaires et non quatre.

### ACC-002 — Numéro alphanumérique

Étant donné le numéro `001a`, lorsque la clé d’appariement est normalisée, alors le résultat conserve `001a` sans le convertir en nombre.

### ACC-003 — Appariement certain

Étant donné une ligne `MOM | 0012` et un seul candidat du catalogue avec cette extension et ce numéro, lorsque l’appariement est exécuté, alors l’état est `Matched` et contient le `uuid` du candidat.

### ACC-004 — Appariement ambigu

Étant donné plusieurs candidats impossibles à départager, lorsque l’appariement est exécuté, alors l’état est `Ambiguous`, aucun `uuid` n’est affecté et les candidats sont conservés pour diagnostic.

### ACC-005 — Prix manquant

Étant donné deux exemplaires tarifés à 150 centimes et une impression sans prix, lorsque la valeur est calculée, alors la valeur est 300 centimes et la couverture signale une impression sans prix.

### ACC-006 — Filtres

Étant donné les couleurs `W` et `U` sélectionnées ainsi que la rareté `rare`, lorsque le filtre est appliqué, alors les cartes blanches ou bleues sont retenues uniquement si elles sont également rares.

### ACC-007 — Échec d’actualisation

Étant donné un catalogue local valide, lorsque le téléchargement du nouveau catalogue échoue, alors l’ancien catalogue reste utilisable et l’erreur est affichée.

### ACC-008 — Désambiguïsation par langue

Étant donné plusieurs impressions ayant le même code d’extension, le même numéro et le même nom, lorsque l’export DragonShield indique `French`, alors l’impression MTGJSON en langue `French` est appariée si elle est unique.

### ACC-009 — Désambiguïsation par finition

Étant donné plusieurs impressions encore possibles après comparaison du nom et de la langue, lorsque l’export DragonShield indique une finition et qu’un seul candidat MTGJSON l’accepte, alors ce candidat est apparié. `Normal` correspond à `nonfoil`, une variante contenant `Etched` correspond à `etched`, et les autres variantes dont le nom se termine par `Foil` correspondent à `foil`.

### ACC-010 — Compatibilité des anciennes impressions DragonShield

Étant donné une impression exportée sous `LEGI`, `DRKI` ou sous un numéro `ARN` terminé par `a`, lorsque l’appariement est exécuté, alors la clé est comparée à l’équivalent MTGJSON `LEG`, `DRK` ou `…†` sans modifier les autres numéros alphanumériques.

### ACC-011 — Prix correspondant à la finition possédée

Étant donné une impression possédée à la fois en normal et en foil, lorsque le guide Cardmarket est importé, alors les exemplaires normaux utilisent `trend` et les exemplaires foil utilisent `trend-foil`, en euros et en centimes.

### ACC-012 — Réappariement après actualisation

Étant donné des lignes non résolues et un nouveau catalogue qui permet désormais de les identifier, lorsque les références sont actualisées, alors ces lignes sont appariées et ajoutées à la collection sans modifier les quantités déjà appariées.

### ACC-013 — Numérotation DragonShield de Burning Inquiry

Étant donné `Burning Inquiry · PLST · 854` dans un export DragonShield et `Burning Inquiry · PLST · M10-128` dans MTGJSON, lorsque l’appariement est exécuté, alors l’impression MTGJSON est sélectionnée.

### ACC-014 — Actualisation quotidienne non bloquante

Étant donné une actualisation déjà réussie aujourd’hui, lorsque l’application redémarre, alors aucun nouveau téléchargement automatique n’est lancé. Lorsque le bouton Actualiser est utilisé, l’actualisation est néanmoins forcée et la navigation reste disponible pendant son exécution.

## 11. Discipline TDD et définition de terminé

Chaque exigence implémentée est reliée à au moins un test portant son identifiant. Le cycle attendu est : test en échec observé, implémentation minimale, tests réussis, puis refactorisation.

Une tranche est terminée lorsque :

- ses critères d’acceptation sont automatisés ;
- `cargo fmt --check` réussit ;
- `cargo clippy --all-targets -- -D warnings` réussit ;
- `cargo test` réussit ;
- les erreurs pertinentes sont couvertes ;
- aucune donnée utilisateur existante n’est détruite lors d’un échec.

## 12. Décisions encore nécessaires

- Règles exactes de langue, finition, foil et variantes disponibles dans l’export DragonShield.
- Comportement détaillé et objectifs de l’aide à la construction de decks.

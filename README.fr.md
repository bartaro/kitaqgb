# KITAQGB

[English](README.md#english) | [日本語](README.md#japanese) | **Français**

**[Manuel du compilateur](https://bartaro.github.io/kitaq-docs/fr/kitaqgb.html)** · **[Manuel de la bibliothèque](https://bartaro.github.io/kitaq-docs/fr/gb-library.html)**

## Origine du nom

KITAQGB est issu d'un fork de NORCAL, le compilateur C pour NES associé à Zachtronics. Il conserve la mention de droit d'auteur de Keith Holman, auteur de NORCAL.

NORCAL tire son nom du nord de la Californie (Northern California). S’inspirant de cette référence géographique, l’auteur a choisi le nom KITAQGB d’après Kitakyushu, la ville où il est né et a grandi. KITAQ + GB associe Game Boy à **北九 (キタキュー, Kitakyū)**, le surnom de Kitakyushu, dans la préfecture de Fukuoka, au Japon. KITAQ se prononce comme le japonais « キタキュー » ; la notation indicative en anglais est **kee-tah-KYOO**, soit **/ˌkiːtɑːˈkjuː/** en alphabet phonétique international. Le Q final se prononce comme le nom anglais de la lettre Q. En anglais, KITAQGB se lit **kee-tah-KYOO jee bee**, en prononçant G et B séparément.

Le nom KITAQGB a deux sens. **Kernel-Informed Toolchain for AI-Quality Game Boy Development** exprime l’objectif d’une chaîne de développement qui comprend la machine cible et accompagne aussi bien les personnes qui programment que l’IA générative.

L’autre sens est **Kids' Imagination Transformed into Actual Quests in Game Boy Forests** : un outil qui transforme l’imagination des enfants en véritables aventures dans les forêts du Game Boy. Il traduit le souhait de faire de petites idées, de croquis et de prototypes créés avec l’aide de l’IA des aventures auxquelles on peut réellement jouer.

## État du projet : version publique préliminaire

KITAQGB et KOKURA sont actuellement proposés comme outils de développement en version publique préliminaire.

Ils peuvent servir à l'expérimentation, aux projets d'exemple, au développement de jeux assisté par IA, à la recherche sur les compilateurs, au débogage par émulation et à la validation des méthodes de travail. Leur développement reste actif : les API, options de commande, formats de sortie, diagnostics et comportements peuvent changer d'une version à l'autre.

Ces versions peuvent contenir des bogues, des fonctions incomplètes ou des changements incompatibles. Vérifiez soigneusement le code généré, le comportement de l'émulateur, les diagnostics temporels et les rapports avant de les utiliser en production ou pour une diffusion publique.

**Kernel-Informed Toolchain for AI-Quality Game Boy Development**

KITAQGB est une chaîne de développement C open source pour créer des logiciels homebrew destinés à la Game Boy et à la Game Boy Color. Elle vise un développement assisté par IA où les diagnostics sont faciles à exploiter : écrire de petits programmes C, les compiler en images ROM `.gb` / `.gbc`, puis utiliser les observations de l'émulateur pour améliorer rapidement le jeu.

KITAQGB n'est ni affilié à Nintendo, ni soutenu, parrainé ou approuvé par Nintendo. Game Boy et Game Boy Color sont des marques de Nintendo.

## Ce qu'est KITAQGB

KITAQGB réunit un compilateur C et des bibliothèques pour le développement homebrew de type Game Boy. Il reprend NORCAL et élargit cette base en une chaîne d'outils adaptée à la création moderne de jeux assistée par IA.

Le projet se concentre sur :

- la compilation du C vers des ROM Game Boy ;
- les logiciels homebrew Game Boy et Game Boy Color ;
- les diagnostics exploitables par l'IA et les rapports de compilation reproductibles ;
- la génération de ROM, d'en-têtes et de code de bas niveau pour les cibles de type LR35902 ;
- les bibliothèques de palettes, tuiles, défilement, caméras, audio, liaison et services RPG/ADV/SLG, notamment pour la Game Boy Color ;
- l'utilisation avec KOKURA CLI pour les tests, les traces et le débogage par émulation.

KITAQGB ne contient **aucune ROM commerciale, aucun BIOS Nintendo, aucun fichier du SDK Nintendo, aucune ressource propriétaire ni aucun matériel de développement officiel Nintendo**.

## Plateformes cibles

KITAQGB produit des ROM homebrew pour :

- les logiciels compatibles Game Boy ;
- les logiciels compatibles Game Boy Color ;
- les logiciels réservés à la Game Boy Color lorsque le projet utilise volontairement ses fonctions propres.

Les fichiers de sortie habituels sont :

```text
*.gb
*.gbc
```

Testez les ROM produites dans un émulateur et, lorsque c'est possible, sur le matériel réel ou avec une cartouche programmable. Le comportement matériel peut être délicat, notamment pour les délais, interruptions, accès VRAM/OAM, sons, communications par câble et commutations de banques.

## Organisation du dépôt

```text
kitaqgb/                  # Racine du dépôt
├─ kitaqgb/               # Sources nécessaires à la compilation du compilateur
│  ├─ *.cs
│  ├─ app.config
│  └─ kitaqgb.csproj
├─ kitaqgb.exe            # Compilateur prêt à l'emploi, en mode Release
├─ kitaqgb.exe.config     # Configuration du runtime .NET Framework
├─ lib/                   # Bibliothèques C
├─ examples/              # Programmes pédagogiques et police originale
├─ scripts/build.ps1      # Reconstruction de l'exécutable Release
├─ LICENSE
└─ LICENSE.ja
```

Le compilateur fourni nécessite Windows avec .NET Framework 4.8. Téléchargez l'archive ZIP du dépôt pour conserver ensemble l'exécutable, sa configuration, les bibliothèques et les mentions de licence. Pour recompiler, il faut également le Developer Pack .NET Framework 4.8 et Visual Studio Build Tools. Depuis la racine du dépôt :

```powershell
.\scripts\build.ps1
.\kitaqgb.exe --help
.\examples\build.ps1
```

Une compilation Release copie l'exécutable et sa configuration à la racine du dépôt. Les compilations Debug restent dans `kitaqgb/bin/Debug` et ne remplacent pas le compilateur Release distribué. Les caches de compilation et les fichiers PDB ne sont pas distribués. Consultez le [compte rendu de compilation binaire](BINARY_BUILD.json) pour les entrées et les empreintes SHA-256.

## Prérequis

L'environnement principal de compilation est :

- Windows ;
- la prise en charge du ciblage .NET Framework 4.8 ;
- Visual Studio ou Visual Studio Build Tools avec MSBuild.

Le fichier de projet est actuellement un projet C# classique ciblant `.NET Framework v4.8`.

Un environnement non Windows peut fonctionner avec Mono/MSBuild selon les assemblies de référence installées, mais Windows avec MSBuild reste la voie principale de compilation prise en charge.

## Compiler KITAQGB

Depuis la racine du dépôt :

```powershell
msbuild kitaqgb\kitaqgb.csproj /p:Configuration=Release
```

Après une compilation réussie, le projet copie l'exécutable à la racine :

```text
kitaqgb.exe
```

Vous pouvez ensuite consulter l'aide en ligne de commande :

```powershell
.\kitaqgb.exe --help
```

## Démarrage rapide

Générez un petit modèle de projet :

```powershell
.\kitaqgb.exe template hello.c --overwrite
```

Compilez-le :

```powershell
.\kitaqgb.exe hello.c -o hello.gb --profile=dev --fast-build --cache
```

Pour une compilation orientée diffusion :

```powershell
.\kitaqgb.exe hello.c -o hello.gb --profile=release --cache
```

Lancez la ROM obtenue dans l'émulateur Game Boy / Game Boy Color de votre choix, ou dans KOKURA CLI si vous utilisez cette démarche de débogage par émulation.

## Utiliser les bibliothèques fournies

Le dossier `lib/` contient du code C réutilisable. Compilez les sources de bibliothèque avec le source du jeu et ajoutez `-I lib` pour trouver leurs en-têtes.

Exemple avec les fonctions d'audio, de palette, de défilement, de caméra et de physique. Les trois commandes suivantes utilisent `cmd.exe` : le caractère `^` y prolonge la ligne.

```cmd
.\kitaqgb.exe main.c ^
  lib\audio_hwregs_gb.c lib\audio.c ^
  lib\cgb_palette.c lib\scroll.c lib\camera.c ^
  lib\physics2d.c lib\physics2d_circle.c lib\physics3d.c ^
  -I lib -o game.gb --profile=dev --fast-build --cache
```

Exemple de projet avec liaison série :

```cmd
.\kitaqgb.exe lib\link_hwregs_gb.c lib\link.c lib\link_packet.c main.c ^
  -I lib -o link_game.gb --profile=dev --fast-build --cache
```

Exemple avec les fonctions RPG / ADV / SLG :

```cmd
.\kitaqgb.exe main.c lib\text.c lib\menu.c lib\flags.c lib\script.c lib\map.c lib\save.c ^
  -I lib -o rpg.gb --profile=dev --fast-build --cache
```

Consultez [le README de la bibliothèque](lib/README.md) pour le classement actuel et les remarques d'utilisation.

## Options courantes

Quelques options utiles :

```text
-o <file>                  Chemin de la ROM de sortie
-I <dir>                   Dossier d'en-têtes
--profile=dev              Profil de développement
--profile=release          Profil de diffusion
--fast-build / --fast      Compilation rapide de développement
--cache                    Activer le cache de compilation
--no-cache                 Désactiver le cache
--disasm                   Produire le désassemblage
--no-disasm                Ne pas produire le désassemblage
--diag-json <file>         Écrire les diagnostics en JSON
--machine-readable         Privilégier une sortie exploitable automatiquement
--deps-out <file>          Produire les informations de dépendances
--debug-output <dir>       Produire les fichiers de débogage et d'assistance
--strict                   Promouvoir certains avertissements en erreurs
--permissive               Assouplir certains diagnostics
--stack-bank=fixed|wramx1  Choisir le modèle de banque de pile
--stack-top=<addr>         Choisir l'adresse du sommet de pile
--stack-reserve=<bytes>    Réserver une zone de pile
```

Pour la liste exacte des options de votre version :

```powershell
.\kitaqgb.exe --help
```

## Travailler avec KOKURA CLI

KITAQGB est conçu pour s'associer à KOKURA CLI, le projet compagnon d'émulation et de débogage.

Une démarche habituelle consiste à :

1. écrire ou générer le code C du jeu ;
2. le compiler avec KITAQGB ;
3. lancer la ROM produite dans KOKURA CLI ;
4. recueillir diagnostics, traces, symboles, observations temporelles et rapports d'émulation ;
5. utiliser ces résultats pour l'itération suivante de code et de débogage.

Cette démarche est particulièrement utile avec l'assistance d'une IA : une erreur de compilation, un rapport d'émulateur ou une trace d'exécution peut devenir une tâche de correction ciblée.

## Principes de développement

KITAQGB n'a pas vocation à devenir un compilateur C moderne généraliste. Il s'agit d'une chaîne de développement homebrew conçue pour une petite plateforme de jeu 8 bits à banques, sensible aux contraintes temporelles.

Le projet privilégie :

- un code généré prévisible ;
- des diagnostics clairs ;
- de petits exemples reproductibles ;
- des rapports de compilation et de débogage lisibles par l'IA ;
- un contrôle de bas niveau lorsque nécessaire ;
- des bibliothèques de haut niveau accessibles lorsque possible.

L'objectif est de rendre le développement pour les machines de type Game Boy plus abordable sans masquer entièrement leur fonctionnement.

<!-- development-prompt:fr:start -->
## Prompt pour développer un jeu

Renseignez les besoins, puis transmettez le prompt complet à votre assistant IA. Il couvre l’implémentation, les tests dans l’émulateur, l’analyse avec SARAKURA et la vérification des corrections.

[Lire l’exemple pratique dans le manuel HTML](https://bartaro.github.io/kitaq-docs/fr/kitaqgb.html#loop-prompts)

<details>
<summary>Afficher le prompt complet</summary>

### Développer un jeu avec KITAQGB, KOKURA et SARAKURA

Renseignez les besoins, puis transmettez ce document entier à l’assistant IA. Les commandes supposent que les dépôts `kitaqgb`, `kitaqfc`, `kokura`, `kurosaki`, `sarakura` et `kitaq-docs`, ainsi que le projet `game-gb` ou `game-fc`, se trouvent dans le même dossier parent. Exécutez-les depuis ce dossier et adaptez les chemins à votre environnement.

#### Besoins

- Titre du jeu : &lt;à renseigner&gt;
- Genre et mécanique principale : &lt;à renseigner&gt;
- Commandes et conditions de réussite ou d’échec : &lt;à renseigner&gt;
- Écrans, niveaux, ennemis et objets indispensables : &lt;à renseigner&gt;
- Style graphique, musique et effets sonores : &lt;à renseigner, avec les chemins des ressources fournies&gt;
- Sauvegarde, communication, périphériques et autres besoins : &lt;à renseigner, ou aucun&gt;
- Dossier du projet : &lt;à renseigner&gt;
- Conditions de redistribution : &lt;par exemple, code et ressources originaux pouvant être publiés sous licence MIT&gt;

- Machine cible : &lt;Game Boy d’origine / compatibilité GB et CGB / CGB uniquement&gt;
- Objectif de performances : &lt;par exemple, 60 mises à jour de la logique par seconde en jeu normal ; préciser ce qui est acceptable dans les scènes exigeantes&gt;

#### Travail demandé

Implémentez le jeu avec KITAQGB et ses bibliothèques. Utilisez KOKURA pour l’exécution et le débogage, et SARAKURA pour organiser les diagnostics et comparer les résultats avant et après correction.

Répétez ce cycle jusqu’à satisfaire les critères d’acceptation : préciser la spécification → implémenter une petite modification → compiler → appliquer des entrées et observer → rechercher la cause → corriger → refaire les tests dans les mêmes conditions. Un plan, du code fourni ou une compilation réussie ne suffisent pas à terminer le travail.

##### Vérifier l’environnement et les critères d’acceptation

1. Lisez les consignes du dossier de travail, les README, les manuels HTML ainsi que les en-têtes et implémentations des bibliothèques utilisées. Relevez les chemins des exécutables et leurs versions ou empreintes SHA-256. Vérifiez les commandes dans la sortie réelle de `--help` et les API dans le code source.
2. Définissez des critères mesurables pour les entrées, l’image, le son, la progression et la fréquence de mise à jour. Par exemple : appuyer puis relâcher START lance la partie ; une collision retire une vie ; la pause coupe les sons prévus et la reprise rétablit la lecture.
3. Ne posez de questions que sur les ambiguïtés importantes. Prenez de façon autonome les décisions courantes et réversibles. Ne réduisez pas les exigences ni les critères d’acceptation.
4. Commencez par faire passer un petit exemple fourni dans le compilateur, l’émulateur et SARAKURA. Cela vérifie leur articulation, pas l’achèvement du jeu demandé.

##### Réaliser une première version jouable

- Utilisez le dialecte C de KITAQGB et `void main()`. Ne supposez pas que les API du C sur ordinateur ou de GBDK sont disponibles. Incluez les implémentations `.c` nécessaires, pas seulement leurs déclarations ; vérifiez initialisation, unités, signe, plages de valeurs, durée de vie des tampons et banques ROM.
- Prévoyez les mises à jour VRAM/OAM, VBlank, interruptions, pile, banques ROM/WRAM et limites de tuiles et de sprites. La capacité totale et libre de la file de transferts est distincte de la capacité et de l’espace libre de la VRAM physique.
- Un jeu DMG ne doit pas dépendre de fonctions réservées au CGB. Si les deux modes sont pris en charge, testez chacun d’eux.
- Utilisez la police originale fournie dans `ascii.c` pour les lettres, chiffres et symboles, et vérifiez la correspondance entre caractères et tuiles.

- Reliez d’abord démarrage, titre, personnage contrôlable, réussite ou échec, et nouvelle partie. Enrichissez ensuite le contenu.
- Conservez les sources modifiables des graphismes, musiques et effets ainsi que les étapes de génération. Vérifiez que la compilation utilise réellement les données exportées.
- Rédigez les commentaires du code en anglais et les comptes rendus d’avancement en français. Gardez les rapports standard de SARAKURA en anglais.

##### Relier chaque compilation à son exécution

Séparez les sorties par itération, par exemple dans `out/iter-001`. Consignez commandes, codes de sortie et empreintes du code, des ressources, outils, ROM et métadonnées. N’exécutez jamais une ancienne ROM après une compilation échouée. Les cartes mémoire, correspondances avec les sources et informations de débogage doivent provenir de la même compilation que la ROM.

Voici une vérification de base pour DMG. Préparez `main.c` et les implémentations de bibliothèque nécessaires ; adaptez les options et la séquence d’entrées au jeu.

```powershell
$iteration = '.\game-gb\out\iter-001'
New-Item -ItemType Directory -Force $iteration | Out-Null

# Include all additional implementation units required by the game.
& '.\kitaqgb\kitaqgb.exe' '.\game-gb\src\main.c' `
  -I '.\kitaqgb\lib' -o "$iteration\game.gb" `
  --profile=dev --rst-disable --stack-bank=fixed --no-disasm `
  "--emit-ai-metadata=$iteration\build.json"
if ($LASTEXITCODE -ne 0) { throw 'Build failed; inspect the build log.' }

# This sequence presses START once, with released intervals on both sides.
& '.\kokura\kokura-cli.exe' "$iteration\game.gb" `
  --hardware dmg --run-frames 300 `
  --input-seq 'NONE:60;START:1;NONE:239' `
  --png "$iteration\frame.png" --record-wav "$iteration\audio.wav" `
  --dump-report "$iteration\run.json" `
  --emit-diagnostics "$iteration\events.jsonl"
if ($LASTEXITCODE -ne 0) { throw 'Emulator run failed; inspect the run log.' }

& '.\sarakura\sarakura.exe' gb analyze `
  --metadata "$iteration\build.json" --events "$iteration\events.jsonl" `
  --frames 300 --out "$iteration\analysis" --fail-on error
if ($LASTEXITCODE -ne 0) { throw 'Inspect the analysis report and fix the cause.' }
```


`--hardware dmg` sélectionne la Game Boy d’origine. Pour tester CGB ou les deux modes, faites correspondre l’en-tête ROM et le matériel choisi dans l’émulateur. La séquence appuie une fois sur START entre deux périodes où les boutons sont relâchés. Une exécution de 300 images ne teste pas l’ensemble du jeu.

##### Vérifier l’image, le son, l’état et les performances

- Conservez les scénarios d’entrée en distinguant pression, maintien et relâchement. Parcourez toutes les voies prévues : démarrage, début de partie, déplacement, actions, collisions, défilement, changement de niveau, fin de partie, redémarrage, pause et, si nécessaire, sauvegarde ou communication.
- Gardez les PNG des images pertinentes, les entrées, rapports d’exécution, diagnostics JSONL, WAV et observations nécessaires d’état ou de mémoire. Vérifiez le nombre d’images atteint et la raison de l’arrêt. Ouvrez réellement les images : une seule capture ne prouve ni mouvement ni réponse aux commandes. Comparez compteurs, positions et transitions aux valeurs attendues ; examinez bords d’écran, limites de tuiles et d’attributs, et scènes chargées en sprites.
- Vérifiez musique, effets, lecture simultanée, coupures, pause et reprise. Produire un WAV ne prouve pas la justesse du son. Si l’écoute est impossible, distinguez les contrôles numériques ou de forme d’onde réalisés des qualités sonores non vérifiées.
- Mesurez les scènes exigeantes, le travail du CPU cible, les mises à jour et les transferts ; sur FC, incluez NMI. Le débit de l’émulateur sur l’ordinateur hôte n’est ni la fréquence de mise à jour du jeu ni une preuve de vitesse sur matériel réel. Poursuivre avec `--allow-unimplemented` ne prouve pas la prise en charge de la fonction manquante.

##### Analyser, corriger et refaire les tests

- Fournissez à SARAKURA les métadonnées de la ROM testée et le JSONL diagnostic de cette exécution. Une trace CPU ou un rapport d’exécution ordinaire ne les remplace pas. `--frames` précise les conditions d’analyse ; SARAKURA n’exécute pas la ROM et ne modifie pas automatiquement les sources.
- Lisez `report.html`, `ai_diagnostics.json`, `repair_prompt.md` et `retest_plan.json`. Confrontez les diagnostics aux étapes de reproduction, images, sons et sources. Distinguez les emplacements ou causes supposés des faits vérifiés, et les boucles d’attente normales des blocages. Examinez chaque avertissement et notez les événements non pris en charge ou les limites d’analyse. Ne masquez pas les avertissements avec des filtres et ne raccourcissez pas les tests pour obtenir un succès.
- Réduisez les défauts à des cas minimaux, corrigez leur cause et recompilez. Si le compilateur ou l’émulateur est en cause, isolez son défaut du code du jeu et ajoutez une vérification de non-régression à la correction de l’outil.
- Refaites les tests avec les mêmes entrées, graine aléatoire, machine et standard vidéo, mapper, images observées et réglages diagnostics. Utilisez les métadonnées propres à chaque ROM ; ne réutilisez pas aveuglément les états sauvegardés après modification du code ou de la disposition RAM.

```powershell
& '.\sarakura\sarakura.exe' baseline-delta `
  --baseline '.\game-gb\out\iter-001\analysis' `
  --current '.\game-gb\out\iter-002\analysis' `
  --out '.\game-gb\out\delta.json' --markdown '.\game-gb\out\delta.md' `
  --fail-on-new error --fail-on-regression error --enforce
```


Associez les différences de diagnostic aux critères d’acceptation des commandes, graphismes et sons. Si le même échec se répète, réexaminez les preuves et l’hypothèse au lieu d’enchaîner des modifications arbitraires.

##### Conditions de fin et livrables

Rejouez tous les scénarios obligatoires avec la ROM finale compilée depuis les sources et réglages livrés. L’invincibilité, des entrées automatiques de test ou un autre mapper ne suffisent pas à valider une partie normale dans la version finale. Fournissez un tableau reliant besoins et tests, expliquez les avertissements restants et indiquez les éléments non vérifiés ou non pris en charge. Précisez explicitement l’absence de tests sur matériel physique, le cas échéant.

Livrez les sources, l’identification des outils et bibliothèques, les ressources modifiables, les scripts reproductibles de compilation et de test, la ROM, les preuves finales et un README expliquant installation, commandes et limites connues. Incluez les replays et le programme de test si nécessaire. Publiez ou transmettez des fichiers à l’extérieur uniquement dans le périmètre expressément autorisé. Supprimez les compilations intermédiaires et traces temporaires inutiles après vérification, mais conservez sources, ressources, livrables et preuves de non-régression nécessaires.

Si l’environnement ou les permissions empêchent un contrôle obligatoire, indiquez les étapes exactes de reproduction et l’action nécessaire. Ne déclarez pas le travail terminé.

</details>
<!-- development-prompt:fr:end -->

## Marques et affiliation

KITAQGB est un projet open source indépendant destiné au développement homebrew.

Il n'est ni affilié à Nintendo, ni soutenu, parrainé ou approuvé par Nintendo. Game Boy et Game Boy Color sont des marques de Nintendo.

N'ajoutez pas à ce dépôt de logos Nintendo, d'illustrations d'emballage officielles, de polices officielles, de BIOS, de données de ROM commerciales ou de ressources de jeux propriétaires sans disposer du droit de le faire.

## Licence

KITAQGB est distribué sous licence MIT.

Le projet dérive de NORCAL, dont la mention d'origine est :

```text
Copyright 2019 Keith Holman
```

Les modifications et ajouts de KITAQGB portent la mention :

```text
Copyright (c) 2026 DAISUKE OBA
```

Les mentions de droit d'auteur et de licence MIT de NORCAL doivent être conservées dans les copies ou parties substantielles du logiciel.

Consultez [LICENSE](LICENSE) et [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) pour les détails.

## Contribuer

Avant de proposer une modification :

- n'ajoutez pas de ROM sous droit d'auteur, de BIOS, de ressources extraites de jeux commerciaux ou de matériel de SDK officiel ;
- évitez d'inclure les sorties générées telles que `bin/`, `obj/`, `target/`, `dist/`, `*.exe`, `*.dll` et `*.pdb` dans les commits de sources, sauf raison précise liée à une diffusion ;
- privilégiez des cas de test petits et reproductibles pour les bogues du compilateur ou de la génération de code ;
- pour une nouvelle bibliothèque, documentez la commande de compilation et les déclarations de registres matériels nécessaires ;
- rédigez des diagnostics assez clairs pour guider aussi bien une personne qu'un agent de programmation IA.

## État de la distribution

Ce dépôt est préparé comme première diffusion publique de KITAQGB. Les interfaces, bibliothèques, diagnostics et intégrations avec les outils compagnons peuvent évoluer avec le projet.

## Compiler et commencer à utiliser l'outil

Utilisez Windows, le Developer Pack .NET Framework 4.8 et Visual Studio Build Tools avec MSBuild. Lancez les commandes depuis Developer PowerShell.

```powershell
MSBuild.exe .\kitaqgb\kitaqgb.csproj /t:Build /p:Configuration=Release
.\kitaqgb.exe --help
.\examples\build.ps1
```

## Manuels et licences

- [Manuel en français](https://bartaro.github.io/kitaq-docs/fr/kitaqgb.html) / [Bibliothèque en français](https://bartaro.github.io/kitaq-docs/fr/gb-library.html)
- [Manuel en anglais](https://bartaro.github.io/kitaq-docs/en/kitaqgb.html) / [Manuel en japonais](https://bartaro.github.io/kitaq-docs/kitaqgb.html)
- [Sources du manuel pour lecture hors connexion](https://github.com/bartaro/kitaq-docs)
- [Licence](LICENSE) / [Traduction japonaise à titre de référence](LICENSE.ja)

La licence du projet ne remplace pas les conditions de tiers relatives aux polices, dépendances, logos ou marques. Conservez les mentions jointes lors de la redistribution.

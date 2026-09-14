# KITAQGB

[English](README.md#english) | [日本語](README.md#japanese) | **Français**

**[Manuel du compilateur](https://bartaro.github.io/kitaq-docs/fr/kitaqgb.html)** · **[Manuel de la bibliothèque](https://bartaro.github.io/kitaq-docs/fr/gb-library.html)**

## Origine du nom

KITAQGB est issu d'un fork de NORCAL, le compilateur C pour NES associé à Zachtronics. Il conserve la mention de droit d'auteur de Keith Holman, auteur de NORCAL.

**NORCAL tire son nom du nord de la Californie (Northern California).** S’inspirant de cette référence géographique, DAISUKE OBA, l’auteur de KITAQGB, a choisi **Kitakyushu**, la ville où il est né et a grandi, pour former le nom KITAQGB.

Le nom **KITAQGB** a deux sens qui se superposent :

- **Kernel-Informed Toolchain for AI-Quality Game Boy Development** exprime l'objectif d'une chaîne de développement qui connaît la machine cible et accompagne aussi bien les programmeurs que les démarches utilisant l'IA générative.
- **KITAQ + GB** associe un nom local de **Kitakyushu**, ville de la **préfecture de Fukuoka, au Japon**, à **Game Boy**. **KITAQ** représente le surnom **北九 (キタキュー, Kitakyū)** de Kitakyushu.

En anglais, **KITAQ** se prononce **« kee-tah-KYOO »**, soit **/ˌkiːtɑːˈkjuː/** en alphabet phonétique international, pour approcher le japonais **キタキュー**. Le **Q** final se prononce comme le nom anglais de la lettre **Q**, « cue ». En anglais, lisez **KITAQGB** « kee-tah-KYOO jee bee », en prononçant **G** et **B** séparément.

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

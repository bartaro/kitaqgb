# GBSEQ — manuel d’utilisation

**2026-11-14 — source update / ソース更新:** Ctrl+K ou l’icône de piano ouvre le clavier avec une sortie audio continue. Nouveau/Ouvrir propose Enregistrer/Ignorer/Annuler dans la fenêtre pour les modifications non enregistrées. Les ROM de lecture permettent de basculer la boucle avec B en SOLO/MASTER ; TIME indique position/durée.

La lecture génère de courts blocs audio à la demande. Le moniteur, le silence et le solo conservent la position ; les notes et le tempo modifiés sont appliqués à la prochaine lecture.

[Audio implementation and source checks](../../SOURCE_UPDATE_20261114.md). The packages in `bin/` retain their recorded build revisions; build the current source for this update.

[Fichiers et périphériques MIDI, clavier, enregistrement et 512 sons](docs/MIDI_PERFORMANCE.md) · `Ctrl+K` · `import-midi` / `export-midi`

Les ROM exportées contiennent les 48 octets du logo Nintendo pour la compatibilité avec la vérification au démarrage. La licence MIT de GBSEQ n’accorde aucun droit indépendant sur le logo ou les marques. GBSEQ n’est ni affilié à Nintendo ni approuvé par Nintendo. [Avis en anglais/japonais](docs/NINTENDO_NOTICE.md).

Dans Exporter, choisissez UGE modifiable (avec vos formes d’onde) ou MOD (instruments GBT fixes). Enregistrez un fichier par console dans `assets/music`. [CLI: `export-uge` / `export-mod`](docs/GB_STUDIO_EXPORT.md).

[English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Español](README.es.md) | [Português](README.pt.md) | [Français](README.fr.md) | [Deutsch](README.de.md)

GBSEQ, par DAISUKE OBA, propose une interface plita et une CLI indépendante. Elles partagent modèle, validation, audio KOKURA et exportateurs. LOCAL 4CH, CABLE 8CH et DMG07 16CH correspondent à 1, 2 et 4 Game Boy. Chaque joueur dispose de CH1/CH2 à impulsions, CH3 à table d’onde et CH4 à bruit. Un morceau contient 1–256 mesures de 16 pas (4096 par canal), BPM 30–300 et une option de boucle. Choisissez la langue dans Affichage → Langue. Ce réglage est enregistré indépendamment du morceau. `--language en|ja|ko|zh-CN|zh-TW|es|pt-BR|fr|de`

## Jeu au clavier, enregistrement et WAV

Sélectionnez une piste et appuyez sur Ctrl+K pour ouvrir le clavier. A/S/D/F/G/H/J jouent les touches blanches, W/E/T/Y/U les noires ; Z/X change l’octave. Cliquez sur IN et OUT pour choisir les ports MIDI. La saisie pas à pas avance de 1, 2, 4 ou 8 doubles croches. REC enregistre une prise depuis le début. La protection refuse les chevauchements ; l’écrasement ne remplace que la partie recouverte. Importez le MIDI depuis Fichier et enregistrez-le depuis Exporter ; gardez le .gbseq pour conserver les timbres GB.

Chaque canal propose 128 timbres d’usine dans la liste déroulante. Vos timbres se sauvegardent et se rechargent séparément au format .gbpreset. L’affichage des accords peut être activé pour le mixage et pour chaque piste indépendamment. La transposition globale ne modifie pas la piste de bruit.

Le WAV respecte l’écoute, les pistes muettes et les solos au lancement de l’export. Tout écouter mélange les consoles actives ; P1–P4 en isole une. En présence d’un solo, seules les pistes en solo non muettes sont audibles. Sélectionner une piste à éditer ne l’isole pas. Rendre toutes les pistes muettes conserve la durée du WAV.

`gbseq-cli render-wav song.gbseq mix.wav` · `gbseq-cli render-wav song.gbseq p2.wav --player 2`

[Exporter le pilote VBlank et la musique en C](docs/KITAQGB_VBLANK.md): `gbseq-cli export-vblank song.gbseq output` · `--strict` · `--driver-dir`.
Les enregistrements suivent l’ordre d’audio.h. Le pilote VBlank ne propose que le triangle et la dent de scie sur CH3. Utilisez --strict pour refuser les approximations et compilez chaque console séparément.

Appuyez sur **E** ou double-cliquez sur l’en-tête d’une piste pour ouvrir son éditeur de notes ; E ramène à la vue d’ensemble. Tirez le bord droit d’une note jusqu’à la note suivante ou à la fin du morceau. Le graphe d’onde du CH3 permet de dessiner les 32 échantillons ; Ctrl+Z annule un trait.

## ROM de lecture et écoute

**Export → Playback ROMs (no SRAM)** crée un fichier de 512 Kio par partie (P1.gb à P4.gb), avec le titre et ses quatre canaux. Aucune sauvegarde n’est nécessaire. START permet d’écouter une partie seule. Pour synchroniser, appuyez sur SELECT à l’arrêt sur chaque console, sur A (MASTER) sur l’une et sur B (PEER) sur les autres ; START sur le maître commande la lecture. Avec l’adaptateur quatre joueurs, le maître se branche sur le port 1, indépendamment de la partie. Dans GBSEQ, écoutez le mixage complet ou une seule console indépendamment de la piste éditée ; l’export WAV suit ce choix.

`gbseq-cli export-player-roms song.gbseq output_folder`

## Installation

Choisissez dans [bin/](bin/) Windows x86_64, Linux x86_64 ou macOS Apple Silicon/Intel. Extrayez toute l’archive et gardez assets et les licences près des exécutables. Lancez gbseq.exe sous Windows ou ./gbseq sous macOS/Linux. Pour la CLI, remplacez gbseq-cli des exemples par ./gbseq-cli.exe sous PowerShell ou ./gbseq-cli sous Unix, sauf si le programme est dans PATH. Aucun compilateur, plita séparé ou Python n’est nécessaire à l’usage ordinaire.

L’interface demande un bureau graphique, un pilote GPU compatible et un périphérique audio pour l’écoute. Linux cible Ubuntu 24.04 ou un système ultérieur compatible, avec ALSA, X11/Wayland, libxkbcommon + libxkbcommon-x11 et Vulkan/OpenGL. Les exécutables macOS ne sont ni signés ni notariés : vérifiez origine et empreinte puis, au besoin, autorisez cette application via Ouvrir/Confidentialité et sécurité, sans désactiver la protection globale. La CLI produit des WAV sans écran ni carte son. Sous Unix, restaurez si nécessaire le droit d’exécution : chmod +x gbseq gbseq-cli.

## Édition graphique

Choisissez le mode et P1–P4. ALL16 en DMG07 affiche les seize canaux. Double-cliquez une piste vide pour créer une note, une note pour ouvrir l’inspecteur. Déplacez son corps pour changer temps/hauteur, son bord droit pour la durée. Chaque piste est monophonique : les chevauchements sont refusés. Réglez 1–256 mesures. Molette/barre inférieure déplacent la vue ; F ajuste au morceau, Home revient au début, +/- zoome. La vue suit la lecture. Space lance/arrête ; LOOP ON/OFF gère la boucle. La lecture génère de courts blocs audio à la demande. Le moniteur, le silence et le solo conservent la position ; les notes et le tempo modifiés sont appliqués à la prochaine lecture.

Ctrl/Shift-clic bascule la sélection ; Ctrl+A sélectionne la piste. Gauche/droite déplace selon la grille ; haut/bas transpose par demi-ton, Shift+haut/bas par octave. Ctrl+D duplique, Delete supprime, G change la grille, Q aligne les débuts. Ctrl+Z/Y annule/rétablit, Ctrl+C/V copie/colle. Ctrl+O ouvre, Ctrl+S enregistre, Ctrl+Shift+S enregistre sous. Le remplacement de modifications non enregistrées demande confirmation ; conservez une copie avant écrasement.

## Timbres et ondes

L’inspecteur règle par note niveau, panoramique gauche/droite/centre, rapport cyclique et enveloppe des impulsions, balayage CH1, niveau/préréglage CH3 et bruit CH4. AUDITION écoute la note. CH3 n’a pas d’enveloppe matérielle. NEW USER WAVE dessine 32 échantillons de 4 bits (0–15). Vous pouvez stocker 64 ondes utilisateur et assigner celles-ci ou sept ondes intégrées à 16 préréglages actifs, niveaux 100/50/25%. WAVE PRESET automatise le timbre par note. Changer l’onde coupe brièvement CH3 ; ce n’est pas un fondu continu.

## CLI

track/player commencent à 1, start à 0. length 1 vaut une double croche, 16 une mesure 4/4. pitch accepte 0–47, velocity 0–15. players accepte 1, 2 ou 4. Les éditions exigent --out, qui peut écraser la destination explicitement choisie. Une édition invalide renvoie un code non nul et n’est pas enregistrée. transform accepte aussi --shift, --quantize et --velocity. Le WAV mélange par défaut toutes les consoles actives ; --player N en isole une. L’export peut remplacer des fichiers existants.

```sh
gbseq --open song.gbseq
gbseq-cli new song.gbseq --bars 32 --bpm 150 --players 4 --no-loop
gbseq-cli add-note song.gbseq --track 1 --start 0 --length 2 --pitch 24 --velocity 12 --out edited.gbseq
gbseq-cli notes edited.gbseq --track 1
gbseq-cli transform edited.gbseq --track 1 --transpose 12 --out higher.gbseq
gbseq-cli duplicate-notes higher.gbseq --ids 1 --out doubled.gbseq
gbseq-cli delete-notes doubled.gbseq --ids 1 --out trimmed.gbseq
gbseq-cli validate edited.gbseq
gbseq-cli info edited.gbseq --json
gbseq-cli render-wav edited.gbseq all16.wav --all-players
gbseq-cli export-kitaqgb edited.gbseq export-c --player 1
gbseq-cli export-link16-sav edited.gbseq saves
gbseq-cli bench-render edited.gbseq --all-players --frames 60 --runs 3
gbseq-cli --help
gbseq-cli transform --help
```


## Export LINK16 et KITAQGB

LINK16 produit des images de sauvegarde MBC5 V4 de 128 Kio : link16_local.sav ; link16_cable.p1.sav/.p2.sav ; ou link16_dmg07.p1.sav–.p4.sav. Utilisez la ROM/version LINK16 correspondante et les noms attendus par l’émulateur. Sauvegardez les anciens SAV avant remplacement. P1 contient aussi les copies des parties distantes. Un SAV ne démarre pas seul. Quatre préréglages d’enveloppe sont partagés par joueur ; les valeurs supplémentaires peuvent être approximées. Écoutez après import. V4 ne conserve pas le balayage CH1. frames_per_step_override permet des effets par image en aperçu/WAV/KITAQGB, mais l’export LINK16 fondé sur le BPM les refuse.

KITAQGB exporte le C/flux d’un joueur : répétez pour chacun. Utilisez audio.h/audio.c compatibles avec CH1=0, CH2=1, CH3=2, CH4=3. Seize canaux ne sont pas fusionnés dans une seule Game Boy. L’aperçu quantifie la durée des pas en images vidéo ; certains BPM diffèrent d’une ROM à horloge fractionnaire. Le rendu émulé n’est pas un test matériel.

## Données et Python

.gbseq est du JSON, pas du MIDI ; renommer l’extension ne convertit rien. Validez les données d’IA et vérifiez durée, pistes actives, identifiants uniques, tessiture et chevauchements. Les [spécifications détaillées](docs/README.md) sont japonaises. Pour l’interface Python 3.10+ facultative, ajoutez python/ à PYTHONPATH et définissez GBSEQ_NATIVE_LIBRARY vers le chemin absolu de gbseq.dll/libgbseq.dylib/libgbseq.so. Les indices Python commencent à zéro ; appelez validate après modification directe de data. player=None mélange tous les joueurs.

```python
from gbseq import Project
song = Project()
song.data["measures"] = 4
song.add_note(track=0, start=0, length=2, pitch=24, velocity=12)
song.validate()
song.save("python-song.gbseq")
song.render_wav("python-song.wav")
```


## Compilation, dépannage et droits

Utilisez Rust 1.93.1 et les outils C/C++ du système. Linux demande libasound2-dev, libudev-dev, libxkbcommon-dev, libxkbcommon-x11-dev, libwayland-dev, libx11-dev, libxi-dev, libxcursor-dev et les bibliothèques de développement Mesa. CI vérifie modèle, édition, audio/SAV et démarrage graphique par OS ; verification enregistre le commit. Ces contrôles ne testent ni sortie audio physique, ni entrée physique, ni Game Boy réelle.

```sh
cd tools/gbseq
cargo test --locked --release --tests
cargo build --locked --release --bin gbseq
cargo build --locked --release --no-default-features --bin gbseq-cli --lib
```


En cas d’échec au lancement, vérifiez le pilote GPU et le terminal. Texte absent : assets/fonts ; silence : périphérique et mute/solo ; export impossible : validate et note signalée. GBSEQ est sous MIT, Copyright (c) 2026 DAISUKE OBA. Conservez [LICENSE](LICENSE), [avis tiers](THIRD_PARTY_NOTICES.md), [audit](LICENSE_AUDIT.json) et licences des polices lors d’une redistribution. Les matériaux tiers et musiques importées gardent leurs propres droits.

# GBSEQ — manuel d’utilisation

[English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Español](README.es.md) | [Português](README.pt.md) | [Français](README.fr.md) | [Deutsch](README.de.md)

GBSEQ, par DAISUKE OBA, propose une interface plita et une CLI indépendante. Elles partagent modèle, validation, audio KOKURA et exportateurs. LOCAL 4CH, CABLE 8CH et DMG07 16CH correspondent à 1, 2 et 4 Game Boy. Chaque joueur dispose de CH1/CH2 à impulsions, CH3 à table d’onde et CH4 à bruit. Un morceau contient 1–256 mesures de 16 pas (4096 par canal), BPM 30–300 et une option de boucle. L’interface est en anglais ; les manuels existent en neuf langues.

## Installation

Choisissez dans [bin/](bin/) Windows x86_64, Linux x86_64 ou macOS Apple Silicon/Intel. Extrayez toute l’archive et gardez assets et les licences près des exécutables. Lancez gbseq.exe sous Windows ou ./gbseq sous macOS/Linux. Pour la CLI, remplacez gbseq-cli des exemples par ./gbseq-cli.exe sous PowerShell ou ./gbseq-cli sous Unix, sauf si le programme est dans PATH. Aucun compilateur, plita séparé ou Python n’est nécessaire à l’usage ordinaire.

L’interface demande un bureau graphique, un pilote GPU compatible et un périphérique audio pour l’écoute. Linux cible Ubuntu 24.04 ou un système ultérieur compatible, avec ALSA, X11/Wayland, libxkbcommon et Vulkan/OpenGL. Les exécutables macOS ne sont ni signés ni notariés : vérifiez origine et empreinte puis, au besoin, autorisez cette application via Ouvrir/Confidentialité et sécurité, sans désactiver la protection globale. La CLI produit des WAV sans écran ni carte son. Sous Unix, restaurez si nécessaire le droit d’exécution : chmod +x gbseq gbseq-cli.

## Édition graphique

Choisissez le mode et P1–P4. ALL16 en DMG07 affiche et joue les seize canaux. Double-cliquez une piste vide pour créer une note, une note pour ouvrir l’inspecteur. Déplacez son corps pour changer temps/hauteur, son bord droit pour la durée. Chaque piste est monophonique : les chevauchements sont refusés. Réglez 1–256 mesures. Molette/barre inférieure déplacent la vue ; F ajuste au morceau, Home revient au début, +/- zoome. La vue suit la lecture. Space lance/arrête ; LOOP ON/OFF gère la boucle. La première lecture calcule tout le morceau et peut demander du temps.

Ctrl/Shift-clic bascule la sélection ; Ctrl+A sélectionne la piste. Gauche/droite déplace selon la grille ; haut/bas transpose par demi-ton, Shift+haut/bas par octave. Ctrl+D duplique, Delete supprime, G change la grille, Q aligne les débuts. Ctrl+Z/Y annule/rétablit, Ctrl+C/V copie/colle. Ctrl+O ouvre, Ctrl+S enregistre, Ctrl+Shift+S enregistre sous. Le remplacement de modifications non enregistrées demande confirmation ; conservez une copie avant écrasement.

## Timbres et ondes

L’inspecteur règle par note niveau, panoramique gauche/droite/centre, rapport cyclique et enveloppe des impulsions, balayage CH1, niveau/préréglage CH3 et bruit CH4. AUDITION écoute la note. CH3 n’a pas d’enveloppe matérielle. NEW USER WAVE dessine 32 échantillons de 4 bits (0–15). Vous pouvez stocker 64 ondes utilisateur et assigner celles-ci ou sept ondes intégrées à 16 préréglages actifs, niveaux 100/50/25%. WAVE PRESET automatise le timbre par note. Changer l’onde coupe brièvement CH3 ; ce n’est pas un fondu continu.

## CLI

track/player commencent à 1, start à 0. length 1 vaut une double croche, 16 une mesure 4/4. pitch accepte 0–47, velocity 0–15. players accepte 1, 2 ou 4. Les éditions exigent --out, qui peut écraser la destination explicitement choisie. Une édition invalide renvoie un code non nul et n’est pas enregistrée. transform accepte aussi --shift, --quantize et --velocity. --all-players mélange tous les joueurs ; par défaut P1 seul. L’export peut remplacer des fichiers existants.

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

LINK16 produit des images de sauvegarde MBC5 V4 de 128 Kio : link16_local.sav ; link16_cable.p1.sav/.p2.sav ; ou link16_dmg07.p1.sav–.p4.sav. Utilisez la ROM/version LINK16 correspondante et les noms attendus par l’émulateur. Sauvegardez les anciens SAV avant remplacement. P1 contient aussi les copies des parties distantes. Aucune ROM LINK16 n’est incluse ; un SAV ne démarre pas seul. Quatre préréglages d’enveloppe sont partagés par joueur ; les valeurs supplémentaires peuvent être approximées. Écoutez après import. V4 ne conserve pas le balayage CH1. frames_per_step_override permet des effets par image en aperçu/WAV/KITAQGB, mais l’export LINK16 fondé sur le BPM les refuse.

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

Utilisez Rust 1.93.1 et les outils C/C++ du système. Linux demande libasound2-dev, libudev-dev, libxkbcommon-dev, libwayland-dev, libx11-dev, libxi-dev, libxcursor-dev et les bibliothèques de développement Mesa. CI vérifie modèle, édition, audio/SAV et démarrage graphique par OS ; verification enregistre le commit. Ces contrôles ne testent ni sortie audio physique, ni entrée physique, ni Game Boy réelle.

```sh
cd tools/gbseq
cargo test --locked --release --tests
cargo build --locked --release --bin gbseq
cargo build --locked --release --no-default-features --bin gbseq-cli --lib
```


En cas d’échec au lancement, vérifiez le pilote GPU et le terminal. Texte absent : assets/fonts ; silence : périphérique et mute/solo ; export impossible : validate et note signalée. GBSEQ est sous MIT, Copyright (c) 2026 DAISUKE OBA. Conservez [LICENSE](LICENSE), [avis tiers](THIRD_PARTY_NOTICES.md), [audit](LICENSE_AUDIT.json) et licences des polices lors d’une redistribution. Les matériaux tiers et musiques importées gardent leurs propres droits.

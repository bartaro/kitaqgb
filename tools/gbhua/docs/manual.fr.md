# GBHUA

Windows 10/11 x64 nécessite aussi [Microsoft Visual C++ v14 Redistributable x64](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist). Si `VCRUNTIME140.dll` manque, installez le paquet x64 actuel de Microsoft. Ne téléchargez pas de DLL isolées sur d'autres sites.

## Téléchargement et démarrage

Téléchargez l'archive adaptée dans [Téléchargements GUI](../GUI.md), puis extrayez-la entièrement. Sous Windows, ouvrez `GBHUA.exe` et gardez le dossier `assets` à côté. Sur Mac, copiez `GBHUA.app` dans Applications ; choisissez Apple Silicon ou Intel. Rust et Python ne sont pas nécessaires pour lancer la GUI. L'application ne dispose ni de signature Developer ID ni de notarisation. Utilisez le dépôt officiel ; si macOS la bloque, vérifiez sa provenance et autorisez uniquement cette application dans Réglages Système > Confidentialité et sécurité. Ne désactivez pas la protection du système.

Les libellés de la GUI sont en anglais ; ce document constitue le manuel traduit. Sur Mac aussi, `Ctrl` désigne Control. Les commandes de compilation partent de la racine du dépôt. La CLI reste utilisable sans écran ni GPU.

[English](manual.en.md) | [日本語](manual.ja.md) | [한국어](manual.ko.md) | [简体中文](manual.zh-CN.md) | [繁體中文](manual.zh-TW.md) | [Español](manual.es.md) | [Português](manual.pt.md) | [Français](manual.fr.md) | [Deutsch](manual.de.md)

GBHUA convertit les PNG en tuiles, palettes et cartes Game Boy. Auteur : DAISUKE OBA. Code original : MIT.

## Compilation et lancement

```powershell
cd tools/gbhua
cargo run --release --locked --features gui --bin gbhua_gui
```

## GUI

Un nouveau document non modifié ne déclenche pas l’avertissement de modifications non enregistrées ; les véritables modifications restent protégées. Sous Windows, la GUI n’ouvre pas de console par défaut. Pour le diagnostic : `cargo run -p gbhua --bin gbhua_gui --features gui-console`.

Les titres et le haut des canevas sont alignés. Survolez les icônes ouvrir, enregistrer et annuler/rétablir pour afficher leur nom. Le bouton de taille au-dessus de Graphic choisit les dimensions ; ajuster, moins et plus au-dessus de Map règlent le zoom. DMG/CGB se trouve en haut à droite. Sous 1180px de largeur, utilisez les onglets Tiles / Palettes à gauche. Parcourez palettes et diagnostics avec la molette ou la barre de défilement. Escape ferme le menu.

1. Choisissez une tuile à gauche et une des sept tailles dans Graphic > 8x8. Les grands graphiques utilisent des tuiles consécutives. Choisissez une couleur, glissez pour peindre ; clic droit pour prélever.

2. TOOLS > FILL remplit une région connexe ; TILES > FILL TILE remplit tout. FLIP X/Y et ROTATE transforment ; EDIT > COPY/PASTE GRAPHIC duplique. UNDO/REDO conserve jusqu'à 64 opérations.

3. TOOLS > MAP place le graphique ; clic droit prélève tuile/palette. RECT place 3x3 ; FILL remplit ; ATTR applique la palette et inverse X ; COLL/EVT active les marques. VIEW propose Fit/1x/2x/4x, grille et marques. Utilisez molette et barres de défilement.

4. Dans BYTECODE, Ctrl+A puis Ctrl+V colle 16 octets. IMPORT modifie la première tuile 8x8 ; EXPORT actualise le texte. Flèches/Home/End déplacent le curseur, Shift sélectionne, Ctrl+C/X copie/coupe, Backspace/Delete supprime.

5. FILE > SAVE et EXPORT demandent une destination. Avec des modifications (*), ouvrir/importer/fermer demande : Yes enregistre, No abandonne, Cancel continue l'édition. L'import PNG remplace le projet et peut être annulé.

6. SPRITES > NEW META crée un métasprite. ADD/DUP/CLEAR FRAME et PREV/NEXT FRAME gèrent les images. En METASPRITE, clic ajoute une pièce, clic droit la retire. DMG / CGB compare DMG/CGB sans modifier les palettes enregistrées.

```text
0x78,0x78,0xCC,0xCC,0xCC,0xCC,0xC0,0xC0,
0xDC,0xDC,0xCC,0xCC,0x7C,0x7C,0x00,0x00
```

Conservez le projet complet en .gbh. Pour les anciens projets JSON, il suffit de renommer l'extension en .gbh ; aucun lecteur de l'ancienne extension n'est fourni. GBTD/GBR/GBTB et GBMB/GBM sont des formats d'échange incomplets. GBMB crée aussi un .gbr de même nom : gardez les deux fichiers ensemble.

## PNG / Python / CLI

Les dimensions doivent être des multiples de 8 (8..2040). --size redimensionne au plus proche voisin. DMG utilise quatre nuances ; CGB approxime RGB555 avec quatre couleurs par tuile et huit palettes au maximum. La transparence est composée sur blanc. Les tuiles identiques sont partagées ; plus de 256 tuiles uniques sont refusées. Vérifiez les avertissements et l'aperçu.

```powershell
python -m venv .venv
.\.venv\Scripts\python -m pip install maturin
.\.venv\Scripts\python -m maturin develop --features python
```

```python
import gbhua
project = gbhua.Project.from_image("artwork.png", mode="cgb")
project.save("scene.gbh")
project.save_preview("preview.png", "cgb", 2)
print(project.validate_json())
```

[CLI: Français](../README.fr.md)

## Importation PNG et dépannage

Utilisez `FILE > IMPORT PNG` ou déposez un PNG. Choisissez DMG/CGB au préalable. L'importation remplace le projet : enregistrez d'abord. Le choix automatique CGB évalue l'erreur de couleur afin de préserver les reflets rares et les contours sombres, dans les limites du Game Boy. Pour récupérer des reflets perdus lors d'une ancienne importation, réimportez le PNG original ; le `.gbh` réduit ne peut pas restituer les pixels perdus. `--size` est une option CLI.

Sélectionnez la tuile, cliquez dans BYTECODE, saisissez exactement 16 octets et appuyez sur IMPORT. EXPORT ne fait qu'actualiser le texte. Si la carte affiche une autre couleur ou orientation, vérifiez palette et attributs de retournement. Le format `.gbh` conserve toutes les données. Les erreurs apparaissent dans la barre d'état ; annuler une boîte de dialogue n'enregistre rien.

Sous macOS/Linux : `python3 -m venv .venv`, `source .venv/bin/activate`, `python -m pip install maturin`, puis `python -m maturin develop --features python`. La compilation des bindings nécessite Python 3.10+ et Rust.

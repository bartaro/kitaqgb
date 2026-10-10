# GBHUA

[English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Español](README.es.md) | [Português](README.pt.md) | [Français](README.fr.md) | [Deutsch](README.de.md)

GBHUA convertit les PNG en tuiles, palettes et cartes Game Boy. Auteur : DAISUKE OBA. Code original : MIT.

## Compilation et lancement

```sh
cd tools/gbhua
cargo build --release --locked
./target/release/gbhua --help
```

Windows: `target\release\gbhua.exe`. Rust >= 1.92.

## Procédure

Importez le PNG, consultez le rapport JSON compact, validez, prévisualisez puis exportez. Chaque commande réussie produit un objet JSON. Pour limiter les jetons, fournissez à l'IA le chemin et le rapport plutôt que le projet complet.

```sh
./target/release/gbhua import-image artwork.png --out scene.gbh --mode cgb --size 160x144
./target/release/gbhua inspect scene.gbh
./target/release/gbhua validate scene.gbh
./target/release/gbhua preview scene.gbh --out preview.png --mode cgb --scale 2
./target/release/gbhua export scene.gbh --out scene.c --prefix scene
./target/release/gbhua export scene.gbh --out tiles.gbtb
./target/release/gbhua export scene.gbh --out world.gbmb
./target/release/gbhua export scene.gbh --out tiles.2bpp
```

Les dimensions doivent être des multiples de 8 (8..2040). --size redimensionne au plus proche voisin. DMG utilise quatre nuances ; CGB approxime RGB555 avec quatre couleurs par tuile et huit palettes au maximum. La transparence est composée sur blanc. Les tuiles identiques sont partagées ; plus de 256 tuiles uniques sont refusées. Vérifiez les avertissements et l'aperçu.

Conservez le projet complet en .gbh. Pour les anciens projets JSON, il suffit de renommer l'extension en .gbh ; aucun lecteur de l'ancienne extension n'est fourni. GBTD/GBR/GBTB et GBMB/GBM sont des formats d'échange incomplets. GBMB crée aussi un .gbr de même nom : gardez les deux fichiers ensemble.

L'écrasement nécessite --force. Les chemins d'entrée et de sortie doivent différer. Codes : 0 succès, 1 projet invalide, 2 erreur d'arguments/E/S/conversion. Aucune API de génération d'images ni transmission réseau. Les dépendances gardent leurs licences : voir LICENSE, THIRD_PARTY_NOTICES.md et DEPENDENCIES.json.

## GUI / Python

L'interface locale et les liaisons Python partagent le moteur. L'interface n'est pas incluse dans le paquet CLI autonome. Dans l'espace local complet, lancez gbhua_gui ; utilisez import gbhua en Python. L'interface propose menus groupés, dialogues de sauvegarde, comparaison DMG/CGB, sept tailles, saisie de 16 octets, annulation/rétablissement, zoom et défilement.


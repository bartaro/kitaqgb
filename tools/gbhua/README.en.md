# GBHUA

[English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Español](README.es.md) | [Português](README.pt.md) | [Français](README.fr.md) | [Deutsch](README.de.md)

GBHUA converts PNG artwork into Game Boy tiles, palettes and maps. Author: DAISUKE OBA. Original code: MIT.

## Build and Run

```sh
cd tools/gbhua
cargo build --release --locked
./target/release/gbhua --help
```

Windows: `target\release\gbhua.exe`. Rust >= 1.92.

## Workflow

Import a PNG, inspect the compact JSON report, validate, preview, then export. Each successful command prints one JSON object; do not paste full project JSON into an AI conversation. Keep the generated PNG in the repository and pass its path.

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

PNG dimensions must be multiples of 8 (8..2040); use --size for nearest-neighbor resizing. DMG uses four shades. CGB approximates RGB555 with four colors per tile and at most eight palettes. Transparency composites over white. Identical tiles are shared; more than 256 unique tiles is rejected. Check warnings and the preview.

Use .gbh for the complete project. Old JSON projects need only their extension renamed to .gbh; there is no old-extension loader. GBTD/GBR/GBTB and GBMB/GBM exchange tiles, palettes and supported map attributes, but not every project field. GBMB output also writes a sibling .gbr: keep both files together.

Existing outputs require --force. Input and output paths must differ. Exit codes: 0 success, 1 invalid project, 2 usage/IO/conversion error. There is no image-generation API or network upload. Dependencies retain their own licenses; see LICENSE, THIRD_PARTY_NOTICES.md and DEPENDENCIES.json.

## GUI / Python

The local GUI and Python bindings share this conversion core. The GUI is not included in the standalone CLI package. In the full local workspace, launch gbhua_gui; import gbhua in Python. The GUI uses grouped menus, save/export dialogs, DMG/CGB comparison, seven graphic sizes, 16-byte tile input, undo/redo and map zoom/scroll.


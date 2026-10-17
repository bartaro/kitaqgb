# GBHUA

## Download and Start

Download the archive for your system from [GUI downloads](../GUI.md). Extract the whole archive. On Windows, open `GBHUA.exe`; keep `assets` next to it. On Mac, copy `GBHUA.app` to Applications and open it. Choose Apple Silicon or Intel to match your Mac. Rust and Python are not needed for these GUI packages. The app is not Developer ID signed or notarized; use only the official repository. If macOS blocks it, review its warning and allow this app individually in System Settings > Privacy & Security only after verifying its source. Do not disable system security.

The GUI uses English labels; this document is the translated manual. `Ctrl` shortcuts refer to the Control key, including on Mac. Source-build commands below run from the repository root. The CLI remains headless by default.

[English](manual.en.md) | [日本語](manual.ja.md) | [한국어](manual.ko.md) | [简体中文](manual.zh-CN.md) | [繁體中文](manual.zh-TW.md) | [Español](manual.es.md) | [Português](manual.pt.md) | [Français](manual.fr.md) | [Deutsch](manual.de.md)

GBHUA converts PNG artwork into Game Boy tiles, palettes and maps. Author: DAISUKE OBA. Original code: MIT.

## Build and Run

```powershell
cd tools/gbhua
cargo run --release --locked --features gui --bin gbhua_gui
```

## GUI

An untouched new document does not show an unsaved-changes warning; real edits still do. On Windows the GUI does not open a console by default. For diagnostics, use `cargo run -p gbhua --bin gbhua_gui --features gui-console`.

Panel headings and canvas tops are aligned. Hover the open, save and undo/redo icons to see their names. Choose dimensions with the size button above Graphic; use fit, minus and plus above Map to change zoom. DMG/CGB is always available at the top right. Below 1180px width, switch the left sidebar between Tiles and Palettes. Scroll palettes and diagnostics with the wheel or scrollbar. Escape closes a menu.

1. Select a tile in the left atlas and a size in Graphic > 8x8 (8x8, 16x16, 32x32, 8x16, 16x8, 16x32, 32x16). Large graphics use consecutive tiles from the selected ID. Choose a swatch, then drag to draw. Right-click picks a color.

2. TOOLS > FILL fills a connected region. TILES > FILL TILE fills the entire graphic. FLIP X/Y and ROTATE transform it. EDIT > COPY GRAPHIC / PASTE GRAPHIC duplicates it. UNDO/REDO restores up to 64 operations.

3. TOOLS > MAP stamps the graphic on the map. Right-click picks tile/palette. RECT stamps 3x3; FILL fills matching cells; ATTR sets palette and toggles X flip; COLL/EVT toggle markers. VIEW sets Fit/1x/2x/4x, grid and marker overlays. Pan with wheel or scrollbars.

4. Click BYTECODE, Ctrl+A, then Ctrl+V to paste sixteen bytes. IMPORT updates the first 8x8 tile; EXPORT updates the text. Arrows/Home/End move the caret, Shift selects, Ctrl+C/X copies/cuts, Backspace/Delete removes adjacent characters.

5. FILE > SAVE always asks for a name. EXPORT asks for a destination for C, PNG, GBR or GBM. Unsaved * prompts on open/import/close: Yes saves, No discards, Cancel keeps editing. PNG import replaces the project and can be undone.

6. SPRITES > NEW META creates a metasprite; ADD FRAME/DUP FRAME/CLEAR FRAME and PREV/NEXT FRAME manage frames. In METASPRITE mode click to place parts, right-click to remove. DMG / CGB compares DMG/CGB without changing stored palettes.

```text
0x78,0x78,0xCC,0xCC,0xCC,0xCC,0xC0,0xC0,
0xDC,0xDC,0xCC,0xCC,0x7C,0x7C,0x00,0x00
```

Use .gbh for the complete project. Old JSON projects need only their extension renamed to .gbh; there is no old-extension loader. GBTD/GBR/GBTB and GBMB/GBM exchange tiles, palettes and supported map attributes, but not every project field. GBMB output also writes a sibling .gbr: keep both files together.

## PNG / Python / CLI

PNG dimensions must be multiples of 8 (8..2040); use --size for nearest-neighbor resizing. DMG uses four shades. CGB approximates RGB555 with four colors per tile and at most eight palettes. Transparency composites over white. Identical tiles are shared; more than 256 unique tiles is rejected. Check warnings and the preview.

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

[CLI: English](../README.en.md)

## PNG Import and Troubleshooting

Use `FILE > IMPORT PNG`, or drop the PNG into the window. Select DMG/CGB before importing. Import replaces the current project; save first. Automatic CGB palette reduction considers color error so rare highlights and dark outlines are not simply discarded in favor of frequent dark colors. Results still obey Game Boy limits. To recover highlights lost by an older import, reimport the original PNG; changing an already reduced `.gbh` cannot recover missing pixels. `--size` is a CLI option, not a GUI field.

If bytecode does not change the tile, select the tile, click inside BYTECODE, replace the text with exactly 16 bytes, and press IMPORT. EXPORT only refreshes the text. If a map cell looks different, check its palette and flip attributes. Save as `.gbh` to retain all editable state. Inspect the status bar for errors; canceling a dialog does not save.

On macOS/Linux, create a virtual environment with `python3 -m venv .venv`, activate it with `source .venv/bin/activate`, then run `python -m pip install maturin` and `python -m maturin develop --features python`. Python 3.10+ and Rust are required only when building bindings.

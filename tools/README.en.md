# Native helper tools

**English** | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Français](README.fr.md) | [Español](README.es.md) | [Português (Brasil)](README.pt.md) | [Deutsch](README.de.md)

The ZX0/VBlank helper executables use Rust. Running them requires neither .NET nor Python/Pillow. Build them with `cargo build --locked --release`; on Windows append `.exe` to each command.


- [GBHUA](gbhua/README.en.md)
- [GBSEQ](gbseq/README.en.md)
- [Manual](https://bartaro.github.io/kitaq-docs/en/kitaqgb.html)

## GBHUA

[GUI downloads](gbhua/GUI.md) / [GUI English manual](gbhua/docs/manual.en.md) / [all nine languages](gbhua/README.md).
Windows, Mac Apple Silicon and Mac Intel GUI packages accompany the shared GUI/CLI sources and Python bindings.
Build the GUI with `cargo run --manifest-path tools/gbhua/Cargo.toml --locked --release --features gui --bin gbhua_gui`.

[GBHUA CLI](gbhua/) converts PNG artwork into GB/DMG and CGB tiles, palettes and maps. It supports `.gbh`, GBTD/GBTB and GBMB exchange, plus C and 2bpp export. Author: DAISUKE OBA. Original code: MIT. No GUI, GPU or Python is required.

[English manual](gbhua/README.en.md) / [manuals in nine languages](gbhua/README.md).
Build this independent package from the repository root with `cargo build --manifest-path tools/gbhua/Cargo.toml --locked --release`.

## ZX0 / VBlank

```text
kitaqgb-patch-vblank --rom game.gb --map game.map
kitaqgb-zx0 input.bin output.zx0
kitaqgb-zx0 output.zx0 restored.bin --decompress
kitaqgb-zx0 input.bin asset.h --header=level_data
kitaqgb-zx0 input.bin output.kqa --format=auto
```

ZX0 accepts 1–65535 input bytes and preserves the C# encoder output. `raw`, count/value `rle`, and the nine-byte `KQA1` automatic container are supported. Equal payload sizes favor raw, then RLE, then ZX0. `--decompress` accepts a bare forward ZX0 v2 stream, with bounded output; backward streams and v1 are unsupported. The format was designed by Einar Saukas; this KITAQ implementation is MIT licensed.

The VBlank patcher edits the specified ROM in place after checking the fixed-bank map symbol and PUSH signature. It updates the header/global checksums. `--no-header-fix` leaves the checksums unchanged; the signature search is a heuristic and does not validate the complete interrupt routine. Keep a backup before patching.

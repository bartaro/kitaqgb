# Native helper tools

[en](README.en.md) · [ja](README.ja.md) · [ko](README.ko.md) · [zh-CN](README.zh-CN.md) · [zh-TW](README.zh-TW.md) · [fr](README.fr.md) · [es](README.es.md) · [de](README.de.md)

All helper executables use Rust. Running them requires neither .NET nor Python/Pillow. Build all executables with `cargo build --locked --release`; on Windows append `.exe` to each command.

```text
kitaqgb-zx0 input.bin output.zx0
kitaqgb-zx0 output.zx0 restored.bin --decompress
kitaqgb-zx0 input.bin asset.h --header=level_data
kitaqgb-zx0 input.bin output.kqa --format=auto
kitaqgb-patch-vblank --rom game.gb --map game.map
```

ZX0 accepts 1–65535 input bytes and preserves the C# encoder output. `raw`, count/value `rle`, and the nine-byte `KQA1` automatic container are supported. Equal payload sizes favor raw, then RLE, then ZX0. `--decompress` accepts a bare forward ZX0 v2 stream, with bounded output; backward streams and v1 are unsupported. The format was designed by Einar Saukas; this KITAQ implementation is MIT licensed.

The VBlank patcher edits the specified ROM in place after checking the fixed-bank map symbol and PUSH signature. It updates the header/global checksums. `--no-header-fix` leaves the checksums unchanged; the signature search is a heuristic and does not validate the complete interrupt routine. Keep a backup before patching.

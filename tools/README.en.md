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

<!-- wire3d-feedback:start -->

Wire3D timing, 88-row profiles and independent clocks

DMG accepts WIRE3D_DMG_HEIGHT 88, 96 or 120; CGB accepts WIRE3DCGB_HEIGHT 88 or 96. Defaults remain DMG 120 and CGB 96. The 88-row viewport is 128×88 with center Y=44. Use the same setting in the library and caller, compiling wire3d_dmg_88.c / wire3d_cgb_88.c instead of the normal entry. DMG 88 shares the 96-row model layout and 16-edge limit. CGB 160×144 mode is unchanged.

[Verification results and examples](https://bartaro.github.io/kitaq-docs/en/gb-library.html#wire3d-feedback-20261009)

Wire3D measurement and regression scripts use Python 3. Emulator checks require the KOKURA Python bridge and C API DLL. Pillow is optional for PNG output.

<!-- wire3d-feedback:end -->

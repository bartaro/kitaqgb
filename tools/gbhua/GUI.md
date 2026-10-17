# GBHUA GUI

Author: DAISUKE OBA. Original application code: MIT.

## Download

| System | Portable package |
| --- | --- |
| Windows x64 | [GBHUA.exe with assets and manuals](https://github.com/bartaro/kitaqgb/raw/refs/heads/main/tools/gbhua/bin/gui/windows-x86_64/gbhua-gui-windows-x86_64.zip) |
| Mac Apple Silicon | [GBHUA.app for arm64](https://github.com/bartaro/kitaqgb/raw/refs/heads/main/tools/gbhua/bin/gui/macos-arm64/gbhua-gui-macos-arm64.tar.gz) |
| Mac Intel | [GBHUA.app for x86_64](https://github.com/bartaro/kitaqgb/raw/refs/heads/main/tools/gbhua/bin/gui/macos-x86_64/gbhua-gui-macos-x86_64.tar.gz) |

Extract the complete archive. Windows: open `GBHUA.exe` and keep `assets` beside
it. Mac: copy `GBHUA.app` to Applications. No Rust or Python installation is needed.

Windows 10/11 x64 also requires [Microsoft Visual C++ v14 Redistributable x64](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist). If `VCRUNTIME140.dll` is reported missing, install the current x64 package from Microsoft. Do not download individual DLLs from other sites.

The Mac build targets macOS 11 or later and a Metal-capable GPU; CI runs on macOS
15, so macOS 11 is a deployment target, not a separately tested OS.

The packages are not Developer ID signed or notarized. Mac bundles have an ad-hoc
signature for binary integrity, not publisher identity. Download only from this
repository and verify the SHA-256 against the adjacent JSON receipt. If macOS
blocks launch, review the warning and allow only this app under System Settings >
Privacy & Security after verifying the source. Do not disable OS security globally.

Each receipt records the exact source commit, target, compiler, archive hash,
per-file hashes, manuals and dependency count. Automated GUI event/layout tests
are not a claim of interactive testing on every Mac or GPU.

## Manuals

[English](docs/manual.en.md) | [日本語](docs/manual.ja.md) | [한국어](docs/manual.ko.md) | [简体中文](docs/manual.zh-CN.md) | [繁體中文](docs/manual.zh-TW.md) | [Español](docs/manual.es.md) | [Português](docs/manual.pt.md) | [Français](docs/manual.fr.md) | [Deutsch](docs/manual.de.md)

All nine guides cover installation, tiles and larger graphics, map placement,
palette/view modes, bytecode import/export, native save dialogs, PNG conversion,
GBTD/GBMB exchange, metasprites, troubleshooting, CLI and Python automation.
The GUI itself currently uses English labels.

## Build from Source

Use Rust 1.93.1 (the tested toolchain) and the platform C/C++ build tools.
Windows requires MSVC Build Tools; macOS requires Xcode Command Line Tools.
Run from the repository root:

```sh
cd tools/gbhua
cargo test --locked --features gui
cargo run --release --locked --features gui --bin gbhua_gui
```

The five required framework crates are curated under `vendor/plita`; no emulator,
ROM or unrelated application source is included. The Mac build enables Metal.
Fonts load from bundled assets even when launched from Finder or another working
directory. `gui-console` is an optional diagnostic feature; Windows normally
opens no console. Existing `PLITA_UI_FONT` overrides remain supported.

Linux source builds require X11/Wayland development packages and a graphics driver;
prebuilt Linux GUI packages are not supplied in this publication.

## Python

Python 3.10 or later and Rust are required for building bindings. In an activated
virtual environment within `tools/gbhua`:

```sh
python -m pip install maturin
python -m maturin develop --features python
```

```python
import gbhua
project = gbhua.Project.from_image("artwork.png", mode="cgb")
project.save("scene.gbh")
project.save_preview("preview.png", "cgb", 2)
print(project.validate_json())
```

Building without features still produces only the headless CLI. GUI and Python
are opt-in; existing CLI commands and `.gbh` projects are unchanged.

## Distribution and Licenses

The [GUI workflow](../../.github/workflows/gbhua-gui.yml) builds on native Windows,
Apple Silicon and Intel runners, runs GUI regression tests and a Python round
trip, and packages all nine guides. `scripts/package_gui.py` includes the
resolved target's registry licenses, Rust standard-library notices, framework
MIT notice, font OFL notices and Lucide ISC/Feather MIT notices. These dependency
licenses are not replaced by the application's MIT license.

Game artwork and imported files retain their own rights. No third-party game
artwork, ROMs, GBTD or GBMB editor binaries are included.

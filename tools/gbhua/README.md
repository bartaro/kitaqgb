# GBHUA

Game Boy tile and map editor, headless CLI and Python bindings.
Author: DAISUKE OBA. Original application code: MIT.

## Desktop GUI

[GUI downloads and build instructions](GUI.md)

GUI manuals: [English](docs/manual.en.md) | [日本語](docs/manual.ja.md) | [한국어](docs/manual.ko.md) | [简体中文](docs/manual.zh-CN.md) | [繁體中文](docs/manual.zh-TW.md) | [Español](docs/manual.es.md) | [Português](docs/manual.pt.md) | [Français](docs/manual.fr.md) | [Deutsch](docs/manual.de.md)

The Windows GUI and Mac application bundles include Roboto, icons, all nine GUI
manuals and dependency notices. GUI labels are English. The full GUI source is
included here; no other local project or private repository is required.

## Headless CLI

[English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Español](README.es.md) | [Português](README.pt.md) | [Français](README.fr.md) | [Deutsch](README.de.md)

Start with the CLI manual for your language. GUI, CLI and Python share the same
conversion core. Default Cargo builds remain headless, without a display or GPU.

Build: `cargo build --release --locked`. Run: `target/release/gbhua --help` (Windows: `target\release\gbhua.exe`).

Prebuilt macOS CLI: [Apple Silicon / Intel downloads and instructions](MACOS.md).
Includes the executable, multilingual manuals and dependency licenses. No Rust or
Python installation is needed. These are CLI executables, not the GUI application.

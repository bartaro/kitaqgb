# KITAQGB

**English** | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Français](README.fr.md) | [Español](README.es.md) | [Português (Brasil)](README.pt-BR.md) | [Deutsch](README.de.md)


## What KITAQGB is

KITAQGB is a C compiler and supporting library set for Game Boy-class homebrew development. It is based on NORCAL and extends that foundation with a larger toolchain-oriented workflow for modern, AI-assisted game creation.

The project focuses on:

- C-to-Game Boy ROM compilation
- Game Boy / Game Boy Color homebrew development
- AI-friendly diagnostics and reproducible build reports
- ROM/header generation and low-level code generation for LR35902-class targets
- Game Boy Color helpers such as palette, tile, scroll, camera, audio, link, and RPG/ADV/SLG support libraries
- Companion use with KOKURA CLI for emulator-based testing, tracing, and debugging

KITAQGB does **not** include commercial ROMs, Nintendo BIOS files, Nintendo SDK files, proprietary assets, or any official Nintendo development material.

## Target platform

KITAQGB targets homebrew ROMs for:

- Game Boy-compatible software
- Game Boy Color-compatible software
- Game Boy Color-specific software, when the project intentionally uses CGB features

Typical output files are:

```text
*.gb
*.gbc
```

The generated ROMs should be tested with an emulator and, where practical, on real hardware or flash cartridge setups. Hardware behavior can be subtle, especially for timing, interrupts, VRAM/OAM access, audio, link cable communication, and bank switching.

## Install

Download and extract the repository ZIP. The supplied compiler and helper executables run without Rust, .NET, Python or Pillow.

Windows executables are at the repository root. Linux and macOS executables are under bin/ in the platform folders listed below. Keep lib/ and license notices with the tools. On Linux/macOS, run chmod +x on the downloaded executables and add their folder to PATH, or invoke them by their full path.

| OS | kitaqgb |
| --- | --- |
| Windows x64 | `./kitaqgb.exe` |
| Linux x64 (static musl) | `bin/linux-x86_64/kitaqgb` |
| Linux x64 (GNU) | `bin/linux-gnu-x86_64/kitaqgb` |
| macOS ARM64 | `bin/macos-arm64/kitaqgb` |
| macOS Intel | `bin/macos-x86_64/kitaqgb` |
| macOS Universal | `bin/macos-universal/kitaqgb` |

## Quick start

Generate a small template project:

```powershell
.\kitaqgb.exe template hello.c --overwrite
```

Compile it:

```powershell
.\kitaqgb.exe hello.c -o hello.gb --profile=dev --fast-build --cache
```

For a release-oriented build:

```powershell
.\kitaqgb.exe hello.c -o hello.gb --profile=release --cache
```

Run the resulting ROM in your preferred Game Boy / Game Boy Color emulator, or in KOKURA CLI if you are using the companion debugger/emulator workflow.

## Using the bundled libraries

The `lib/` directory contains reusable C support code. Compile the library source files together with your game source and add `-I lib` so headers can be found.

Example with audio, palette, scrolling, camera, and physics helpers:

```cmd
.\kitaqgb.exe main.c ^
  lib\audio_hwregs_gb.c lib\audio.c ^
  lib\cgb_palette.c lib\scroll.c lib\camera.c ^
  lib\physics2d.c lib\physics2d_circle.c lib\physics3d.c ^
  -I lib -o game.gb --profile=dev --fast-build --cache
```

Example for serial-link projects:

```cmd
.\kitaqgb.exe lib\link_hwregs_gb.c lib\link.c lib\link_packet.c main.c ^
  -I lib -o link_game.gb --profile=dev --fast-build --cache
```

Example for RPG / ADV / SLG helper projects:

```cmd
.\kitaqgb.exe main.c lib\text.c lib\menu.c lib\flags.c lib\script.c lib\map.c lib\save.c ^
  -I lib -o rpg.gb --profile=dev --fast-build --cache
```

See `lib/README.md` for the current library classification and notes.

### Build from source

Build the compiler and all helper tools on Windows, Linux, macOS ARM or macOS Intel with Rust 1.85 or later. The native executables run without .NET; production asset tools also run without Python or Pillow.

```sh
cargo build --locked --release
```

```powershell
.\scripts\build.ps1
.\kitaqgb.exe --help
```

```sh
sh scripts/build.sh
```

[Native Rust compiler and helper tools](tools/README.en.md)

## 3D rendering

Wire3D timing, 88-row profiles and independent clocks

DMG accepts WIRE3D_DMG_HEIGHT 88, 96 or 120; CGB accepts WIRE3DCGB_HEIGHT 88 or 96. Defaults remain DMG 120 and CGB 96. The 88-row viewport is 128×88 with center Y=44. Use the same setting in the library and caller, compiling wire3d_dmg_88.c / wire3d_cgb_88.c instead of the normal entry. DMG 88 shares the 96-row model layout and 16-edge limit. CGB 160×144 mode is unchanged.

[Examples](https://bartaro.github.io/kitaq-docs/en/gb-library.html#wire3d-feedback-20261009)

## Common command-line options

Useful options include:

```text
-o <file>                  Output ROM path
-I <dir>                   Include directory
--profile=dev              Development profile
--profile=release          Release profile
--fast-build / --fast      Faster development build path
--cache                    Enable build cache
--no-cache                 Disable build cache
--disasm                   Emit disassembly output
--no-disasm                Suppress disassembly output
--diag-json <file>         Write diagnostics as JSON
--machine-readable         Prefer machine-readable output
--deps-out <file>          Emit dependency information
--debug-output <dir>       Emit debug/helper outputs
--strict                   Promote selected warnings to errors
--permissive               Relax selected diagnostics
--stack-bank=fixed|wramx1  Select stack bank model
--stack-top=<addr>         Select stack top address
--stack-reserve=<bytes>    Reserve stack area
```

For the exact option list supported by your build, run:

```powershell
.\kitaqgb.exe --help
```

## KOKURA CLI companion workflow

KITAQGB is designed to pair naturally with KOKURA CLI, an emulator/debugger-oriented companion project.

A typical workflow is:

```text
1. Write or generate C game code
2. Compile it with KITAQGB
3. Run the generated ROM in KOKURA CLI
4. Capture diagnostics, traces, symbols, timing observations, and emulator reports
5. Feed the results back into the next code/debug iteration
```

This workflow is especially useful for AI-assisted development, where a compiler error, emulator report, or runtime trace can be turned into a focused repair task.

## Repository layout

```text
Cargo.toml / Cargo.lock
src/                     # Rust compiler and native helper sources
kitaqgb.exe             # Windows x64 compiler
kitaqgb-*.exe           # Windows native helper tools
bin/                     # Linux and macOS executables
lib/                     # C libraries for console ROMs
tests/                   # Frozen reference fixtures and Rust tests
scripts/build.ps1
scripts/build.sh
```

## Development philosophy

KITAQGB is not intended to be a general-purpose modern C compiler. It is a purpose-built homebrew toolchain for a small, timing-sensitive, banked 8-bit game platform.

The project values:

- predictable generated code
- clear diagnostics
- small reproducible examples
- AI-readable build and debug reports
- low-level control when needed
- friendly high-level helper libraries when possible

In other words, KITAQGB aims to make Game Boy-class development more approachable without hiding the machine completely.

## Game development prompt

[Manual](https://bartaro.github.io/kitaq-docs/en/loop-engineering.html#gb)

## Name Origin

KITAQGB began as a fork of NORCAL, the NES C compiler associated with Zachtronics. It retains the copyright notice of NORCAL's original author, Keith Holman.

NORCAL takes its name from Northern California. Inspired by this geographical naming, the author named KITAQGB after Kitakyushu, the city where they were born and raised. KITAQ + GB combines Game Boy with KITAQ, the nickname of Kitakyushu in Fukuoka Prefecture, Japan: **北九 (キタキュー, Kitakyū)**. Pronounce KITAQ as **kee-tah-KYOO**, IPA **/ˌkiːtɑːˈkjuː/**. The final Q sounds like the English letter Q. Read KITAQGB as **kee-tah-KYOO jee bee**, pronouncing G and B separately.

The name KITAQGB has two meanings. **Kernel-Informed Toolchain for AI-Quality Game Boy Development** expresses the goal of a toolchain that understands its target machine and supports both human programmers and generative AI.

The other meaning is **Kids' Imagination Transformed into Actual Quests in Game Boy Forests**: a tool that turns children’s imagination into real adventures in the forests of Game Boy. It expresses the creative wish to turn small ideas, sketches and AI-assisted prototypes into adventures people can actually play.

## Contributing

Before submitting changes, please keep the following rules in mind:

- Do not add copyrighted ROMs, BIOS files, extracted commercial assets, or official SDK material.
- Keep generated build outputs such as `bin/`, `obj/`, `target/`, `dist/`, `*.exe`, `*.dll`, and `*.pdb` out of source commits unless there is a specific release reason.
- Prefer small, reproducible test cases for compiler or code generation bugs.
- When adding helper libraries, document the expected compile command and required hardware register declarations.
- Keep diagnostics clear enough for both humans and AI coding agents to act on.

## Project Status: Public Preview

KITAQGB and KOKURA are currently available as public preview development tools.

They are usable for experimentation, sample projects, AI-assisted game development, compiler research, emulator-based debugging, and workflow validation. However, the projects are under active development, and APIs, CLI options, output formats, diagnostics, and behavior may change between versions.

Preview builds may contain bugs, incomplete features, or breaking changes. Generated code, emulator behavior, timing diagnostics, and reports should be verified carefully before use in production or public releases.

**Kernel-Informed Toolchain for AI-Quality Game Boy Development**

KITAQGB is an open-source C toolchain for developing homebrew software targeting the Game Boy and Game Boy Color hardware. It is designed for AI-assisted, diagnostics-friendly development: write small C programs, compile them into `.gb` / `.gbc` ROM images, and use emulator-side feedback to improve the game quickly.

KITAQGB is not affiliated with, endorsed by, sponsored by, or approved by Nintendo.
Game Boy and Game Boy Color are trademarks of Nintendo.

## Manuals and licenses

- [Japanese HTML manuals](https://bartaro.github.io/kitaq-docs/kitaqgb.html) / [English manuals](https://bartaro.github.io/kitaq-docs/en/kitaqgb.html)
- [Offline manual source](https://github.com/bartaro/kitaq-docs)
- [License](LICENSE) / [Japanese reference translation](LICENSE.ja)

The project license does not replace third-party font, dependency, logo or trademark terms. Preserve the accompanying notices when redistributing.

---

## Trademark and affiliation notice

KITAQGB is an independent open-source project for homebrew development.

KITAQGB is not affiliated with, endorsed by, sponsored by, or approved by Nintendo.
Game Boy and Game Boy Color are trademarks of Nintendo.

Do not use Nintendo logos, official packaging art, official fonts, BIOS files, commercial ROM data, or proprietary game assets in this repository unless you have the legal right to do so.

## License

KITAQGB is distributed under the MIT License.

KITAQGB is derived from NORCAL, originally:

```text
Copyright 2019 Keith Holman
```

KITAQGB modifications and additions are:

```text
Copyright (c) 2026 DAISUKE OBA
```

The original NORCAL copyright notice and MIT license notice must be preserved in copies or substantial portions of the software.

See `LICENSE` and `THIRD_PARTY_NOTICES.md` for details.

[Manual](https://bartaro.github.io/kitaq-docs/en/kitaqgb.html)

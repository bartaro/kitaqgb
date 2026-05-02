# KITAQGB

**Kernel-Informed Toolchain for AI-Quality Game Boy Development**

KITAQGB is an open-source C toolchain for developing homebrew software targeting the Game Boy and Game Boy Color hardware. It is designed for AI-assisted, diagnostics-friendly development: write small C programs, compile them into `.gb` / `.gbc` ROM images, and use emulator-side feedback to improve the game quickly.

KITAQGB is not affiliated with, endorsed by, sponsored by, or approved by Nintendo.
Game Boy and Game Boy Color are trademarks of Nintendo.

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

## Name origin

The name **KITAQGB** stands for:

> **Kernel-Informed Toolchain for AI-Quality Game Boy Development**

The name reflects the goal of building a toolchain that understands the target machine deeply enough to support both human programmers and generative AI workflows. In practical terms, KITAQGB is meant to make small Game Boy / Game Boy Color game projects easier to generate, compile, inspect, debug, and iterate.

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

## Repository layout

A recommended repository layout is:

```text
KITAQGB/
├─ README.md
├─ LICENSE
├─ THIRD_PARTY_NOTICES.md
├─ README_RELEASE_NOTES.md
├─ .gitignore
├─ kitaqgb/              # C# compiler/toolchain source
│  ├─ *.cs
│  └─ kitaqgb.csproj
└─ lib/                  # C helper libraries for homebrew projects
   ├─ audio.c / audio.h
   ├─ cgb_palette.c / cgb_palette.h
   ├─ cgb_tile.h
   ├─ scroll.c / scroll.h
   ├─ camera.c / camera.h
   ├─ link.c / link.h
   ├─ rpg.h
   └─ ...
```

If KOKURA CLI is distributed in the same GitHub repository, a monorepo layout such as the following is also suitable:

```text
KITAQGB/
├─ README.md
├─ LICENSE
├─ THIRD_PARTY_NOTICES.md
├─ kitaqgb/              # KITAQGB compiler/toolchain
├─ lib/                  # KITAQGB C libraries
└─ kokura/               # Optional KOKURA CLI emulator/debugger companion
```

## Requirements

Primary build environment:

- Windows
- .NET Framework 4.8 targeting support
- Visual Studio or Visual Studio Build Tools with MSBuild

The project file is currently a classic C# project targeting `.NET Framework v4.8`.

Non-Windows environments may work with Mono/MSBuild depending on the installed reference assemblies, but the primary supported build path is Windows + MSBuild.

## Building KITAQGB

From the repository root:

```powershell
msbuild kitaqgb\kitaqgb.csproj /p:Configuration=Release
```

The project copies the built executable to the repository root after a successful build:

```text
kitaqgb.exe
```

You can then check the command-line help:

```powershell
.\kitaqgb.exe --help
```

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

```powershell
.\kitaqgb.exe main.c ^
  lib\audio_hwregs_gb.c lib\audio.c ^
  lib\cgb_palette.c lib\scroll.c lib\camera.c ^
  lib\physics2d.c lib\physics2d_circle.c lib\physics3d.c ^
  -I lib -o game.gb --profile=dev --fast-build --cache
```

Example for serial-link projects:

```powershell
.\kitaqgb.exe lib\link_hwregs_gb.c lib\link.c lib\link_packet.c main.c ^
  -I lib -o link_game.gb --profile=dev --fast-build --cache
```

Example for RPG / ADV / SLG helper projects:

```powershell
.\kitaqgb.exe main.c lib\text.c lib\menu.c lib\flags.c lib\script.c lib\map.c lib\save.c ^
  -I lib -o rpg.gb --profile=dev --fast-build --cache
```

See `lib/README.md` for the current library classification and notes.

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

## Contributing

Before submitting changes, please keep the following rules in mind:

- Do not add copyrighted ROMs, BIOS files, extracted commercial assets, or official SDK material.
- Keep generated build outputs such as `bin/`, `obj/`, `target/`, `dist/`, `*.exe`, `*.dll`, and `*.pdb` out of source commits unless there is a specific release reason.
- Prefer small, reproducible test cases for compiler or code generation bugs.
- When adding helper libraries, document the expected compile command and required hardware register declarations.
- Keep diagnostics clear enough for both humans and AI coding agents to act on.

## Status

This repository is prepared as an initial public release of KITAQGB. Interfaces, helper libraries, diagnostics, and companion-tool integration may evolve as the project matures.

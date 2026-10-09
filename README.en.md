# KITAQGB

<!-- readme-language-links:start -->
[English](README.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Français](README.fr.md) | [Español](README.es.md) | [Deutsch](README.de.md)
<!-- readme-language-links:end -->

<!-- manual-language-links:start -->
| Language / 言語 | HTML |
| --- | --- |
| English | [kitaqgb](https://bartaro.github.io/kitaq-docs/en/kitaqgb.html) |
| 日本語 | [kitaqgb](https://bartaro.github.io/kitaq-docs/kitaqgb.html) |
| 한국어 | [kitaqgb](https://bartaro.github.io/kitaq-docs/ko/kitaqgb.html) |
| 简体中文 | [kitaqgb](https://bartaro.github.io/kitaq-docs/zh-CN/kitaqgb.html) |
| 繁體中文 | [kitaqgb](https://bartaro.github.io/kitaq-docs/zh-TW/kitaqgb.html) |
| Français | [kitaqgb](https://bartaro.github.io/kitaq-docs/fr/kitaqgb.html) |
| Español | [kitaqgb](https://bartaro.github.io/kitaq-docs/es/kitaqgb.html) |
| Deutsch | [kitaqgb](https://bartaro.github.io/kitaq-docs/de/kitaqgb.html) |
<!-- manual-language-links:end -->

<!-- rust-native-20261004:start -->
## Native Rust compiler and helper tools

Build the compiler and all helper tools on Windows, Linux, macOS ARM or macOS Intel with Rust 1.85 or later. The native executables run without .NET; production asset tools also run without Python or Pillow.

```sh
cargo test --locked --tests
cargo build --locked --release
```

Windows executables are at the repository root. Linux and macOS executables are under bin/ in the platform folders listed below. Keep lib/ and license notices with the tools. On Linux/macOS, run chmod +x on the downloaded executables and add their folder to PATH, or invoke them by their full path.

| OS | kitaqgb |
| --- | --- |
| Windows x64 | `./kitaqgb.exe` |
| Linux x64 (static musl) | `bin/linux-x86_64/kitaqgb` |
| Linux x64 (GNU) | `bin/linux-gnu-x86_64/kitaqgb` |
| macOS ARM64 | `bin/macos-arm64/kitaqgb` |
| macOS Intel | `bin/macos-x86_64/kitaqgb` |
| macOS Universal | `bin/macos-universal/kitaqgb` |

```powershell
.\scripts\build.ps1
.\kitaqgb.exe --help
```

```sh
sh scripts/build.sh
```


Native builds and executable checks passed on Windows, Linux, macOS ARM and macOS Intel. KITAQGB passed 48 tests and 395 helper checks per platform; KITAQFC passed 55 tests and 401 helper checks. Rust 1.85 was also tested. PUBLIC_DISTRIBUTION.json records the installed binary hashes and validation provenance. The public GitHub Actions workflows rebuild and test this source independently.

Frozen reference outputs test ROM bytes, diagnostics and helper formats. Earlier C# emulator evidence remains historical evidence with its original source fingerprints. It does not automatically prove every Rust API, real hardware or complete FDS BIOS/game startup. The unavailable original PNG conversion script was reconstructed from its specification; byte parity with that missing script cannot be claimed.

[Native Rust compiler and helper tools](tools/README.en.md)

<!-- rust-native-20261004:end -->






<a name="english"></a>

## English

### Name Origin

KITAQGB began as a fork of NORCAL, the NES C compiler associated with Zachtronics. It retains the copyright notice of NORCAL's original author, Keith Holman.

NORCAL takes its name from Northern California. Inspired by this geographical naming, the author named KITAQGB after Kitakyushu, the city where they were born and raised. KITAQ + GB combines Game Boy with KITAQ, the nickname of Kitakyushu in Fukuoka Prefecture, Japan: **北九 (キタキュー, Kitakyū)**. Pronounce KITAQ as **kee-tah-KYOO**, IPA **/ˌkiːtɑːˈkjuː/**. The final Q sounds like the English letter Q. Read KITAQGB as **kee-tah-KYOO jee bee**, pronouncing G and B separately.

The name KITAQGB has two meanings. **Kernel-Informed Toolchain for AI-Quality Game Boy Development** expresses the goal of a toolchain that understands its target machine and supports both human programmers and generative AI.

The other meaning is **Kids' Imagination Transformed into Actual Quests in Game Boy Forests**: a tool that turns children’s imagination into real adventures in the forests of Game Boy. It expresses the creative wish to turn small ideas, sketches and AI-assisted prototypes into adventures people can actually play.

### Project Status: Public Preview

KITAQGB and KOKURA are currently available as public preview development tools.

They are usable for experimentation, sample projects, AI-assisted game development, compiler research, emulator-based debugging, and workflow validation. However, the projects are under active development, and APIs, CLI options, output formats, diagnostics, and behavior may change between versions.

Preview builds may contain bugs, incomplete features, or breaking changes. Generated code, emulator behavior, timing diagnostics, and reports should be verified carefully before use in production or public releases.

**Kernel-Informed Toolchain for AI-Quality Game Boy Development**

KITAQGB is an open-source C toolchain for developing homebrew software targeting the Game Boy and Game Boy Color hardware. It is designed for AI-assisted, diagnostics-friendly development: write small C programs, compile them into `.gb` / `.gbc` ROM images, and use emulator-side feedback to improve the game quickly.

KITAQGB is not affiliated with, endorsed by, sponsored by, or approved by Nintendo.
Game Boy and Game Boy Color are trademarks of Nintendo.

### What KITAQGB is

KITAQGB is a C compiler and supporting library set for Game Boy-class homebrew development. It is based on NORCAL and extends that foundation with a larger toolchain-oriented workflow for modern, AI-assisted game creation.

The project focuses on:

- C-to-Game Boy ROM compilation
- Game Boy / Game Boy Color homebrew development
- AI-friendly diagnostics and reproducible build reports
- ROM/header generation and low-level code generation for LR35902-class targets
- Game Boy Color helpers such as palette, tile, scroll, camera, audio, link, and RPG/ADV/SLG support libraries
- Companion use with KOKURA CLI for emulator-based testing, tracing, and debugging

KITAQGB does **not** include commercial ROMs, Nintendo BIOS files, Nintendo SDK files, proprietary assets, or any official Nintendo development material.


### Target platform

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

### Repository layout

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

Build the compiler and all helper tools on Windows, Linux, macOS ARM or macOS Intel with Rust 1.85 or later. The native executables run without .NET; production asset tools also run without Python or Pillow.

```sh
cargo test --locked --tests
cargo build --locked --release
```

Windows executables are at the repository root. Linux and macOS executables are under bin/ in the platform folders listed below. Keep lib/ and license notices with the tools. On Linux/macOS, run chmod +x on the downloaded executables and add their folder to PATH, or invoke them by their full path.

| OS | kitaqgb |
| --- | --- |
| Windows x64 | `./kitaqgb.exe` |
| Linux x64 (static musl) | `bin/linux-x86_64/kitaqgb` |
| Linux x64 (GNU) | `bin/linux-gnu-x86_64/kitaqgb` |
| macOS ARM64 | `bin/macos-arm64/kitaqgb` |
| macOS Intel | `bin/macos-x86_64/kitaqgb` |
| macOS Universal | `bin/macos-universal/kitaqgb` |

```powershell
.\scripts\build.ps1
.\kitaqgb.exe --help
```

```sh
sh scripts/build.sh
```

### Requirements

Build the compiler and all helper tools on Windows, Linux, macOS ARM or macOS Intel with Rust 1.85 or later. The native executables run without .NET; production asset tools also run without Python or Pillow.

```sh
cargo test --locked --tests
cargo build --locked --release
```

Windows executables are at the repository root. Linux and macOS executables are under bin/ in the platform folders listed below. Keep lib/ and license notices with the tools. On Linux/macOS, run chmod +x on the downloaded executables and add their folder to PATH, or invoke them by their full path.

| OS | kitaqgb |
| --- | --- |
| Windows x64 | `./kitaqgb.exe` |
| Linux x64 (static musl) | `bin/linux-x86_64/kitaqgb` |
| Linux x64 (GNU) | `bin/linux-gnu-x86_64/kitaqgb` |
| macOS ARM64 | `bin/macos-arm64/kitaqgb` |
| macOS Intel | `bin/macos-x86_64/kitaqgb` |
| macOS Universal | `bin/macos-universal/kitaqgb` |

```powershell
.\scripts\build.ps1
.\kitaqgb.exe --help
```

```sh
sh scripts/build.sh
```

### Building KITAQGB

Build the compiler and all helper tools on Windows, Linux, macOS ARM or macOS Intel with Rust 1.85 or later. The native executables run without .NET; production asset tools also run without Python or Pillow.

```sh
cargo test --locked --tests
cargo build --locked --release
```

Windows executables are at the repository root. Linux and macOS executables are under bin/ in the platform folders listed below. Keep lib/ and license notices with the tools. On Linux/macOS, run chmod +x on the downloaded executables and add their folder to PATH, or invoke them by their full path.

| OS | kitaqgb |
| --- | --- |
| Windows x64 | `./kitaqgb.exe` |
| Linux x64 (static musl) | `bin/linux-x86_64/kitaqgb` |
| Linux x64 (GNU) | `bin/linux-gnu-x86_64/kitaqgb` |
| macOS ARM64 | `bin/macos-arm64/kitaqgb` |
| macOS Intel | `bin/macos-x86_64/kitaqgb` |
| macOS Universal | `bin/macos-universal/kitaqgb` |

```powershell
.\scripts\build.ps1
.\kitaqgb.exe --help
```

```sh
sh scripts/build.sh
```

### Quick start

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

### Using the bundled libraries

For compressed tile and map assets, see the [ZX0 API and visual example](https://bartaro.github.io/kitaq-docs/en/gb-library.html#module-zx0) and the [host compression tool](tools/zx0/README.md#english).

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

### Common command-line options

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

### KOKURA CLI companion workflow

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

### Development philosophy

KITAQGB is not intended to be a general-purpose modern C compiler. It is a purpose-built homebrew toolchain for a small, timing-sensitive, banked 8-bit game platform.

The project values:

- predictable generated code
- clear diagnostics
- small reproducible examples
- AI-readable build and debug reports
- low-level control when needed
- friendly high-level helper libraries when possible

In other words, KITAQGB aims to make Game Boy-class development more approachable without hiding the machine completely.

<!-- development-prompt:en:start -->
### Game development prompt

Fill in the requirements, then give the complete prompt to your AI assistant. It covers implementation, emulator testing, SARAKURA analysis and retesting.

[Read the reference example in the HTML manual](https://bartaro.github.io/kitaq-docs/en/kitaqgb.html#loop-prompts)

<details>
<summary>Show the complete prompt</summary>

#### Game development with KITAQGB, KOKURA and SARAKURA

Fill in the requirements and give this entire document to the AI assistant. Commands assume sibling repositories named `kitaqgb`, `kitaqfc`, `kokura`, `kurosaki`, `sarakura` and `kitaq-docs`, with your `game-gb` or `game-fc` project beside them. Run commands from their parent directory; adapt paths to the actual environment.

##### Requirements

- Game title: &lt;fill in&gt;
- Genre and core gameplay: &lt;fill in&gt;
- Controls, success and failure conditions: &lt;fill in&gt;
- Required screens, stages, enemies and items: &lt;fill in&gt;
- Visual style, music and sound effects: &lt;fill in; identify any supplied assets&gt;
- Saving, communication, peripherals and other requirements: &lt;fill in, or none&gt;
- Project directory: &lt;fill in&gt;
- Redistribution requirements: &lt;for example, original code and assets suitable for MIT publication&gt;

- Target: &lt;original Game Boy / dual GB–CGB support / CGB only&gt;
- Performance: &lt;for example, 60 gameplay updates per second in normal play; define acceptable behavior in demanding scenes&gt;

##### Task

Implement the game with KITAQGB and its libraries. Use KOKURA for execution and debugging, and SARAKURA to organize diagnostics and compare results before and after a fix.

Repeat this cycle until the acceptance criteria are met: make the specification concrete → implement a small change → build → apply inputs and observe → investigate the cause → fix → retest under the same conditions. A plan, a code listing or a successful compilation is not completion.

###### Establish the environment and acceptance criteria

1. Read workspace instructions, tool READMEs, HTML manuals, and the headers and implementations of the libraries you will use. Record executable paths and versions or SHA-256 hashes. Verify commands against actual `--help` output and APIs against source.
2. Define measurable acceptance criteria for inputs, images, audio, progression and update frequency. Examples: pressing and releasing START begins the game; a collision removes one life; pausing silences the intended audio and resuming restores playback.
3. Ask only about material ambiguities. Make ordinary reversible implementation decisions autonomously. Do not weaken requirements or acceptance criteria.
4. First run a small supplied sample through the compiler, emulator and SARAKURA. This checks the tool connection, not completion of the requested game.

###### Implement a small playable slice

- Use the KITAQGB C dialect and `void main()`. Do not assume desktop C or GBDK APIs are available. Include required `.c` implementation units, not just declarations; check initialization order, units, signedness, ranges, buffer lifetime and ROM banking.
- Plan VRAM/OAM updates, VBlank, interrupts, stack, ROM/WRAM banks and tile/sprite limits. Transfer-queue capacity and free space are different from physical VRAM capacity and free space.
- A DMG game must not depend on CGB-only features. Test both hardware modes for a dual-mode game.
- Use the supplied original `ascii.c` font for letters, digits and symbols, and verify character-to-tile mapping.

- First connect boot, title, a controllable player, success or failure, and restart. Then expand the game.
- Keep editable graphics, music and sound-effect sources and their generation steps. Verify that the build actually consumes their exports.
- Write source comments in English and progress reports in English. Keep SARAKURA’s standard reports in English.

###### Connect each build to its execution

Use a separate output directory for each iteration, such as `out/iter-001`. Record commands, exit codes, and hashes of source, assets, tools, ROM and metadata. Never run an older ROM after a failed build. Maps, source maps and debug information must come from the same build as the ROM.

The following is a basic DMG check. Supply `main.c` and every required library implementation unit, and adapt the options and input sequence to the game.

```powershell
$iteration = '.\game-gb\out\iter-001'
New-Item -ItemType Directory -Force $iteration | Out-Null

# Include all additional implementation units required by the game.
& '.\kitaqgb\kitaqgb.exe' '.\game-gb\src\main.c' `
  -I '.\kitaqgb\lib' -o "$iteration\game.gb" `
  --profile=dev --rst-disable --stack-bank=fixed --no-disasm `
  "--emit-ai-metadata=$iteration\build.json"
if ($LASTEXITCODE -ne 0) { throw 'Build failed; inspect the build log.' }

# This sequence presses START once, with released intervals on both sides.
& '.\kokura\kokura-cli.exe' "$iteration\game.gb" `
  --hardware dmg --run-frames 300 `
  --input-seq 'NONE:60;START:1;NONE:239' `
  --png "$iteration\frame.png" --record-wav "$iteration\audio.wav" `
  --dump-report "$iteration\run.json" `
  --emit-diagnostics "$iteration\events.jsonl"
if ($LASTEXITCODE -ne 0) { throw 'Emulator run failed; inspect the run log.' }

& '.\sarakura\sarakura.exe' gb analyze `
  --metadata "$iteration\build.json" --events "$iteration\events.jsonl" `
  --frames 300 --out "$iteration\analysis" --fail-on error
if ($LASTEXITCODE -ne 0) { throw 'Inspect the analysis report and fix the cause.' }
```


`--hardware dmg` selects the original GB. Match the ROM header and emulator hardware setting when testing CGB or dual support. The input sequence presses START once between released intervals. A 300-frame run does not test the whole game.

###### Check images, audio, state and performance

- Save input scenarios with distinct presses, holds and releases. Exercise every specified path: boot, start, movement, actions, collisions, scrolling, stage changes, game over, restart, pause and saving or communication where applicable.
- Preserve PNGs at relevant frames, input data, execution reports, diagnostic JSONL, WAVs and any necessary state or memory observations. Check the reached frame count and stop reason. Actually open the images; one screenshot cannot establish motion or input response. Compare counters, positions and state changes with expected values. Check screen edges, tile/attribute boundaries and crowded sprite scenes.
- Check music, effects, simultaneous playback, dropouts, pause and resume. A generated WAV alone does not establish correct sound. If listening is unavailable, distinguish the waveform/numerical checks performed from unverified audible qualities.
- Measure heavy scenes, target CPU/update workload and transfers; on FC, include NMI work. Host emulator throughput is not game update frequency or proof of hardware speed. Continuing with `--allow-unimplemented`, where available, does not demonstrate support for the missing feature.

###### Analyze, repair and retest

- Feed SARAKURA the build metadata for the tested ROM and diagnostic JSONL from the tested execution. A CPU trace or ordinary run report is not a substitute. `--frames` specifies analysis conditions; SARAKURA does not execute the ROM or automatically edit the source.
- Read `report.html`, `ai_diagnostics.json`, `repair_prompt.md` and `retest_plan.json`. Compare diagnoses with reproduction steps, images, audio and source. Distinguish inferred source locations or causes from verified facts, and normal waiting loops from hangs. Assess warnings individually and record unsupported events or analysis limits. Do not hide warnings with filters or shorten tests to obtain a passing result.
- Reduce failures to minimal reproductions, fix their causes and rebuild. If the compiler or emulator is responsible, isolate its defect from game code and add regression verification for the tool fix.
- Retest with matching input, random seed, hardware/video mode, mapper, observed frames and diagnostic settings. Use new metadata for each new ROM; do not blindly reuse save states after code or RAM layout changes.

```powershell
& '.\sarakura\sarakura.exe' baseline-delta `
  --baseline '.\game-gb\out\iter-001\analysis' `
  --current '.\game-gb\out\iter-002\analysis' `
  --out '.\game-gb\out\delta.json' --markdown '.\game-gb\out\delta.md' `
  --fail-on-new error --fail-on-regression error --enforce
```


Use diagnostic differences alongside gameplay, graphics and audio acceptance checks. If the same failure repeats, revisit the evidence and hypothesis instead of continuing arbitrary changes.

###### Completion and deliverables

Rerun all required scenarios against the final ROM built from the delivered source and settings. Invincibility, automatic test input or another mapper alone does not verify normal play in the final build. Provide a requirement-to-test table, reasons for remaining warnings and explicit unverified or unsupported items. State “not tested on physical hardware” when applicable.

Deliver source, tool/library identities, editable assets, reproducible build and test scripts, the ROM, final verification evidence, and a README covering setup, controls and known limits. Include replay data and a test harness where needed. Publish or send files externally only within explicitly authorized scope. Delete unnecessary intermediate builds and temporary traces after verification, preserving source, assets, final deliverables and needed regression evidence.

If environment or permission constraints prevent a required check, report the exact reproduction steps and required action. Do not mark the work complete.

</details>
<!-- development-prompt:en:end -->

### Trademark and affiliation notice

KITAQGB is an independent open-source project for homebrew development.

KITAQGB is not affiliated with, endorsed by, sponsored by, or approved by Nintendo.
Game Boy and Game Boy Color are trademarks of Nintendo.

Do not use Nintendo logos, official packaging art, official fonts, BIOS files, commercial ROM data, or proprietary game assets in this repository unless you have the legal right to do so.

### License

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

### Contributing

Before submitting changes, please keep the following rules in mind:

- Do not add copyrighted ROMs, BIOS files, extracted commercial assets, or official SDK material.
- Keep generated build outputs such as `bin/`, `obj/`, `target/`, `dist/`, `*.exe`, `*.dll`, and `*.pdb` out of source commits unless there is a specific release reason.
- Prefer small, reproducible test cases for compiler or code generation bugs.
- When adding helper libraries, document the expected compile command and required hardware register declarations.
- Keep diagnostics clear enough for both humans and AI coding agents to act on.

### Status

This repository is prepared as an initial public release of KITAQGB. Interfaces, helper libraries, diagnostics, and companion-tool integration may evolve as the project matures.

### Build and first use

Build the compiler and all helper tools on Windows, Linux, macOS ARM or macOS Intel with Rust 1.85 or later. The native executables run without .NET; production asset tools also run without Python or Pillow.

```sh
cargo test --locked --tests
cargo build --locked --release
```

Windows executables are at the repository root. Linux and macOS executables are under bin/ in the platform folders listed below. Keep lib/ and license notices with the tools. On Linux/macOS, run chmod +x on the downloaded executables and add their folder to PATH, or invoke them by their full path.

| OS | kitaqgb |
| --- | --- |
| Windows x64 | `./kitaqgb.exe` |
| Linux x64 (static musl) | `bin/linux-x86_64/kitaqgb` |
| Linux x64 (GNU) | `bin/linux-gnu-x86_64/kitaqgb` |
| macOS ARM64 | `bin/macos-arm64/kitaqgb` |
| macOS Intel | `bin/macos-x86_64/kitaqgb` |
| macOS Universal | `bin/macos-universal/kitaqgb` |

```powershell
.\scripts\build.ps1
.\kitaqgb.exe --help
```

```sh
sh scripts/build.sh
```

### Manuals and licenses

- [Japanese HTML manuals](https://bartaro.github.io/kitaq-docs/kitaqgb.html) / [English manuals](https://bartaro.github.io/kitaq-docs/en/kitaqgb.html)
- [Offline manual source](https://github.com/bartaro/kitaq-docs)
- [License](LICENSE) / [Japanese reference translation](LICENSE.ja)

The project license does not replace third-party font, dependency, logo or trademark terms. Preserve the accompanying notices when redistributing.

---


<!-- wire3d-feedback:start -->

Wire3D timing, 88-row profiles and independent clocks

DMG accepts WIRE3D_DMG_HEIGHT 88, 96 or 120; CGB accepts WIRE3DCGB_HEIGHT 88 or 96. Defaults remain DMG 120 and CGB 96. The 88-row viewport is 128×88 with center Y=44. Use the same setting in the library and caller, compiling wire3d_dmg_88.c / wire3d_cgb_88.c instead of the normal entry. DMG 88 shares the 96-row model layout and 16-edge limit. CGB 160×144 mode is unchanged.

[Verification results and examples](https://bartaro.github.io/kitaq-docs/en/gb-library.html#wire3d-feedback-20261009)

<!-- wire3d-feedback:end -->

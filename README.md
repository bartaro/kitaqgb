# KITAQGB

<!-- manual-language-links:start -->
| Language / 言語 | HTML |
| --- | --- |
| English | [KITAQGB](https://bartaro.github.io/kitaq-docs/en/kitaqgb.html) · [KITAQGB Library](https://bartaro.github.io/kitaq-docs/en/gb-library.html) |
| 日本語 | [KITAQGB](https://bartaro.github.io/kitaq-docs/kitaqgb.html) · [KITAQGB Library](https://bartaro.github.io/kitaq-docs/gb-library.html) |
<!-- manual-language-links:end -->


[English](#english) | [日本語](#japanese)

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
kitaqgb/                  # Repository root
├─ kitaqgb/               # Compiler build sources
│  ├─ *.cs
│  ├─ app.config
│  └─ kitaqgb.csproj
├─ kitaqgb.exe            # Prebuilt Release compiler
├─ kitaqgb.exe.config     # .NET Framework runtime configuration
├─ lib/                # C support libraries
├─ examples/           # Tutorial programs and original font
├─ scripts/build.ps1   # Rebuild the Release executable
├─ LICENSE
└─ LICENSE.ja
```

The prebuilt compiler requires Windows with .NET Framework 4.8. Download the
repository ZIP to keep the executable, runtime configuration, libraries and
license notices together. Rebuilding additionally requires the .NET Framework
4.8 Developer Pack and Visual Studio Build Tools. From the repository root:

```powershell
.\scripts\build.ps1
.\kitaqgb.exe --help
.\examples\build.ps1
```

A Release build copies the executable and its configuration to the repository
root. Debug builds stay inside `kitaqgb/bin/Debug` and do not overwrite the
distributed Release compiler. Build caches and PDB files are not distributed.
See [binary build record](BINARY_BUILD.json) for the build inputs and SHA-256.

### Requirements

Primary build environment:

- Windows
- .NET Framework 4.8 targeting support
- Visual Studio or Visual Studio Build Tools with MSBuild

The project file is currently a classic C# project targeting `.NET Framework v4.8`.

Non-Windows environments may work with Mono/MSBuild depending on the installed reference assemblies, but the primary supported build path is Windows + MSBuild.

### Building KITAQGB

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

Windows, .NET Framework 4.8 Developer Pack and Visual Studio Build Tools (MSBuild). Run from a Developer PowerShell prompt.

```powershell
MSBuild.exe .\kitaqgb\kitaqgb.csproj /t:Build /p:Configuration=Release
.\kitaqgb.exe --help
.\examples\build.ps1
```

### Manuals and licenses

- [Japanese HTML manuals](https://bartaro.github.io/kitaq-docs/kitaqgb.html) / [English manuals](https://bartaro.github.io/kitaq-docs/en/kitaqgb.html)
- [Offline manual source](https://github.com/bartaro/kitaq-docs)
- [License](LICENSE) / [Japanese reference translation](LICENSE.ja)

The project license does not replace third-party font, dependency, logo or trademark terms. Preserve the accompanying notices when redistributing.

---

<a name="japanese"></a>

## 日本語

### 名称の由来

KITAQGBはZachtronicsに関連するNES用CコンパイラNORCALを出発点とするフォークです。原作者Keith Holman氏の著作権表示を保持しています。

NORCALの名前は北カリフォルニア（Northern California）に由来します。地域の名前を使うこの命名に着想を得て、作者は自身が生まれ育った北九州市をもとにKITAQGBと名付けました。KITAQ + GB、つまり日本の福岡県北九州市の愛称である北九（キタキュー、Kitakyū）とGame Boyを組み合わせた意味です。KITAQは「キタキュー」と読みます。英語話者向けの発音の目安は kee-tah-KYOO、発音記号は /ˌkiːtɑːˈkjuː/ です。最後のQは英語のアルファベットQと同じ音です。KITAQGBはGとBを一文字ずつ読み、「キタキュー・ジー・ビー」と呼びます。

KITAQGBという名前には、二つの意味があります。**Kernel-Informed Toolchain for AI-Quality Game Boy Development**は、対象マシンを理解し、人間のプログラマーと生成AIの双方を支えるツールチェーンという目標を表しています。

もう一つの意味は、**Kids' Imagination Transformed into Actual Quests in Game Boy Forests**です。「子どもの想像を、Game Boyの森で本物の冒険へ変える道具」という意味を込めています。小さなアイデアや落書き、AI支援による試作を、実際に遊べる冒険へ変えていくための道具にしたい、という創作面での願いを表しています。

### 公開状況：パブリックプレビュー

KITAQGBとKOKURAは、現在パブリックプレビューの開発ツールとして公開しています。

実験、サンプル制作、AIを活用したゲーム開発、コンパイラの研究、エミュレータ上のデバッグ、開発手順の検証に利用できます。開発は継続中で、API、CLIオプション、出力形式、診断内容、動作はバージョンによって変わる場合があります。

プレビュー版には不具合、未完成の機能、互換性のない変更が含まれる可能性があります。製品や公開作品に使う前に、生成コード、エミュレータの動作、タイミング診断、レポートを十分に検証してください。

**Kernel-Informed Toolchain for AI-Quality Game Boy Development**

KITAQGBは、Game BoyおよびGame Boy Color向けの自作ソフトウェアを開発するオープンソースのCツールチェーンです。AIによる開発支援と診断情報の活用を想定しています。小さなCプログラムを書き、`.gb` / `.gbc` ROMへコンパイルし、エミュレータから得た情報を使ってゲームを改善できます。

KITAQGBは任天堂との提携関係になく、同社の推奨、後援、承認を受けたものではありません。Game BoyおよびGame Boy Colorは任天堂の商標です。

### KITAQGBとは

KITAQGBは、Game Boy系の自作ソフトウェア向けCコンパイラと支援ライブラリです。NORCALを基礎に、AIを活用した現代的なゲーム制作のための開発手順とツール連携を拡張しています。

主な対象は次のとおりです。

- CソースからGame Boy ROMへのコンパイル
- Game Boy / Game Boy Color向けの自作ソフトウェア開発
- AIが扱いやすい診断情報と再現可能なビルドレポート
- ROM・ヘッダー生成とLR35902系ターゲット向けの低水準コード生成
- Game Boy Color用のパレット、タイル、スクロール、カメラ、音声、通信、RPG・ADV・SLG支援ライブラリ
- KOKURA CLIと連携したエミュレータ上のテスト、トレース、デバッグ

市販ROM、任天堂のBIOS・SDK、権利者が専有する素材、任天堂の公式開発資料は同梱していません。


### 対象プラットフォーム

次の自作ROMを対象としています。

- Game Boy互換ソフトウェア
- Game Boy Color互換ソフトウェア
- CGB機能を意図的に使用するGame Boy Color専用ソフトウェア

一般的な出力ファイルは次の形式です。

```text
*.gb
*.gbc
```

生成したROMはエミュレータでテストし、可能であれば実機やフラッシュカートリッジでも確認してください。特にタイミング、割り込み、VRAM・OAMアクセス、音声、通信ケーブル、バンク切り替えでは、ハードウェア固有の細かな動作が関係します。

### リポジトリの構成

```text
kitaqgb/                  # リポジトリのルート
├─ kitaqgb/               # コンパイラのビルド用ソース
│  ├─ *.cs
│  ├─ app.config
│  └─ kitaqgb.csproj
├─ kitaqgb.exe            # ビルド済みRelease版コンパイラ
├─ kitaqgb.exe.config     # .NET Frameworkの実行設定
├─ lib/                   # C支援ライブラリ
├─ examples/              # 入門プログラムと自作フォント
├─ scripts/build.ps1      # Release版の再ビルド
├─ LICENSE
└─ LICENSE.ja
```

ビルド用のC#ソースとプロジェクトは `kitaqgb/` にまとめています。ビルド済みRelease版はリポジトリ直下の `kitaqgb.exe` です。実行にはWindowsと.NET Framework 4.8が必要です。リポジトリのZIPを取得すると、実行ファイル、設定ファイル、ライブラリ、権利表記をまとめて入手できます。

再ビルドには.NET Framework 4.8 Developer PackとVisual Studio Build Toolsも必要です。リポジトリ直下で実行してください。

```powershell
.\scripts\build.ps1
.\kitaqgb.exe --help
.\examples\build.ps1
```

Releaseビルドでは実行ファイルと設定ファイルをリポジトリ直下にコピーします。Debugビルドは `kitaqgb/bin/Debug` に置かれ、配布用Release版を上書きしません。ビルドキャッシュとPDBファイルは配布していません。ビルド入力とSHA-256は[バイナリのビルド記録](BINARY_BUILD.json)を参照してください。

### 必要な環境

主なビルド環境は次のとおりです。

- Windows
- .NET Framework 4.8をターゲットにするための開発環境
- MSBuildを含むVisual StudioまたはVisual Studio Build Tools

現在のプロジェクトファイルは `.NET Framework v4.8` を対象とするMSBuild形式のC#プロジェクトです。

Windows以外でも、参照アセンブリの構成によってはMono/MSBuildで動作する可能性があります。ただし、主にサポートするビルド手順はWindowsとMSBuildの組み合わせです。

### KITAQGBのビルド

リポジトリ直下で実行します。

```powershell
msbuild kitaqgb\kitaqgb.csproj /p:Configuration=Release
```

ビルドが成功すると、実行ファイルがリポジトリ直下へコピーされます。

```text
kitaqgb.exe
```

コマンドラインのヘルプを確認できます。

```powershell
.\kitaqgb.exe --help
```

### 最初のプログラム

小さなテンプレートを生成します。

```powershell
.\kitaqgb.exe template hello.c --overwrite
```

コンパイルします。

```powershell
.\kitaqgb.exe hello.c -o hello.gb --profile=dev --fast-build --cache
```

リリース向けのビルドは次のとおりです。

```powershell
.\kitaqgb.exe hello.c -o hello.gb --profile=release --cache
```

生成したROMをGame Boy / Game Boy Colorエミュレータで実行します。付属ツールと連携する開発手順ではKOKURA CLIを利用できます。

### 同梱ライブラリの利用

タイルやマップを圧縮して使う場合は、[ZX0のAPIと画面付きの使用例](https://bartaro.github.io/kitaq-docs/gb-library.html#module-zx0)、[PC側の圧縮ツール](tools/zx0/README.md#日本語)を参照してください。

`lib/` には再利用可能なCの支援コードがあります。必要なライブラリのソースをゲームのソースと一緒にコンパイルし、ヘッダーの検索用に `-I lib` を指定してください。

音声、パレット、スクロール、カメラ、物理演算を使う例です。以下の3例は `^` で行を継続するため、コマンドプロンプト（cmd.exe）で実行します。

```cmd
.\kitaqgb.exe main.c ^
  lib\audio_hwregs_gb.c lib\audio.c ^
  lib\cgb_palette.c lib\scroll.c lib\camera.c ^
  lib\physics2d.c lib\physics2d_circle.c lib\physics3d.c ^
  -I lib -o game.gb --profile=dev --fast-build --cache
```

シリアル通信を使う例です。

```cmd
.\kitaqgb.exe lib\link_hwregs_gb.c lib\link.c lib\link_packet.c main.c ^
  -I lib -o link_game.gb --profile=dev --fast-build --cache
```

RPG・ADV・SLG用の支援機能を使う例です。

```cmd
.\kitaqgb.exe main.c lib\text.c lib\menu.c lib\flags.c lib\script.c lib\map.c lib\save.c ^
  -I lib -o rpg.gb --profile=dev --fast-build --cache
```

現在のライブラリ分類と注意事項は `lib/README.md` を参照してください。

### よく使うコマンドラインオプション

```text
-o <file>                  出力ROMのパス
-I <dir>                   インクルード検索フォルダー
--profile=dev              開発用プロファイル
--profile=release          リリース用プロファイル
--fast-build / --fast      開発時の高速ビルド
--cache                    ビルドキャッシュを有効化
--no-cache                 ビルドキャッシュを無効化
--disasm                   逆アセンブル結果を出力
--no-disasm                逆アセンブル結果の出力を抑止
--diag-json <file>         診断情報をJSONで出力
--machine-readable         機械が読みやすい出力を優先
--deps-out <file>          依存関係を出力
--debug-output <dir>       デバッグ・補助ファイルを出力
--strict                   一部の警告をエラーとして扱う
--permissive               一部の診断を緩和
--stack-bank=fixed|wramx1  スタックのバンク構成を選択
--stack-top=<addr>         スタック最上位アドレスを指定
--stack-reserve=<bytes>    スタック領域を予約
```

使用しているビルドが対応する正確な一覧は、次のコマンドで確認してください。

```powershell
.\kitaqgb.exe --help
```

### KOKURA CLIと連携する開発手順

KITAQGBは、エミュレータ・デバッガであるKOKURA CLIと組み合わせて使うことを想定しています。

```text
1. Cでゲームコードを書く、または生成する
2. KITAQGBでコンパイルする
3. 生成したROMをKOKURA CLIで実行する
4. 診断、トレース、シンボル、タイミング観測、エミュレータのレポートを取得する
5. 結果を次のコード修正・デバッグに反映する
```

コンパイルエラー、エミュレータのレポート、実行時トレースを具体的な修正作業に変えられるため、AIを活用した開発にも適しています。

### 開発方針

KITAQGBは汎用の現代的なCコンパイラを目指すものではありません。小規模で、タイミング制約とバンク構成を持つ8ビットゲーム機の自作開発に特化しています。

次の点を重視しています。

- 動作を予測しやすい生成コード
- 明確な診断情報
- 小さく再現可能なサンプル
- AIが読めるビルド・デバッグレポート
- 必要に応じた低水準の制御
- 利用しやすい高水準の支援ライブラリ

マシンの仕組みを完全に隠すことなく、Game Boy系の開発に取り組みやすくすることを目指しています。

<!-- development-prompt:ja:start -->
### ゲーム開発プロンプト

依頼内容を記入して、プロンプト全文を生成AIに渡してください。実装、エミュレータ検証、SARAKURA解析、修正後の再検証まで含みます。

[HTMLマニュアルの参考例を読む](https://bartaro.github.io/kitaq-docs/kitaqgb.html#loop-prompts)

<details>
<summary>プロンプト全文を表示</summary>

#### KITAQGB・KOKURA・SARAKURAによるゲーム開発プロンプト

以下の「依頼内容」を記入し、このファイル全体を生成AIに渡してください。
コマンドは、`kitaqgb`、`kokura`、`sarakura`、`kitaq-docs`、`game-gb` が同じ親フォルダーにある配置を前提にしています。既存環境では実際のパスを使ってください。

##### 依頼内容

- ゲーム名：〈記入〉
- ジャンル・遊びの中心となる仕組み：〈記入〉
- プレイヤーが行う操作と、成功・失敗条件：〈記入〉
- 必須の画面・ステージ・敵・アイテム：〈記入〉
- 見た目、BGM、効果音：〈記入。資料がある場合はファイルも指定〉
- 対象機種：〈初代Game Boy／GB・CGB両対応／CGB専用〉
- 性能目標：〈例：通常時に毎秒60回のゲーム更新。重い場面の許容条件も記入〉
- 保存・通信・その他の要件：〈記入。不要なら「なし」〉
- プロジェクトの保存先：〈例：game-gb〉
- 再配布条件：〈例：自作コードと素材をMITで公開できる状態にする〉

##### あなたに実行してほしいこと

KITAQGBと付属ライブラリで、上記のゲームを実装してください。
デバッグと実行検証にはKOKURA、診断の整理と修正前後の比較にはSARAKURAを使います。
「仕様を具体化 → 小さく実装 → ビルド → 操作して観測 → 原因を調べる → 修正 → 同条件で再検証」を、受け入れ条件を満たすまで繰り返してください。計画、コードの提示、コンパイル成功だけで完了にしないでください。

###### 1. 環境と受け入れ条件を確定する

1. 作業先の指示、各ツールのREADME、対象言語のHTMLマニュアル、使用するライブラリのヘッダーと実装を読んでください。実行ファイルの場所・バージョンまたはSHA-256を記録し、コマンドとAPIは実際の `--help` とソースで確認してください。
2. 使用機種、ROMの構成、入力、画面、音、更新頻度について、合格・不合格を判断できる受け入れ条件を書いてください。例えば「STARTを押して離すとタイトルからゲームが始まる」「衝突で残機が1減る」「ポーズ中は指定どおりの無音になり、解除後に音楽が再開する」のように具体化してください。
3. 重要な仕様の曖昧さだけを確認し、通常の可逆な実装判断は自律的に進めてください。仕様や合格基準を勝手に弱めないでください。
4. まず付属の小さなサンプルでコンパイラ・KOKURA・SARAKURAの接続を確認してください。これを依頼されたゲームの完成と扱わないでください。

###### 2. 小さく遊べる単位で実装する

- 最初に「起動・タイトル・操作可能なプレイヤー・成功または失敗・再開」までをつなぎ、その後に内容を増やしてください。
- KITAQGBのC方言に合わせ、GBのエントリーポイントは `void main()` を使ってください。一般のデスクトップCやGBDKの関数を、そのまま利用可能だと仮定しないでください。
- 宣言だけでなく対応する実装ファイルも確認し、必要な `.c` をビルド対象へ含めてください。初期化順序、値の単位、符号、範囲、バッファ寿命、ROMバンクを確認してください。
- VRAM・OAM更新、VBlank、割り込み、スタック、ROM/WRAMバンク、タイル・スプライトの上限を設計に含めてください。VRAM転送キューの総容量・空き容量は、物理VRAMの空き容量とは別です。
- DMGを対象にする場合はCGB専用機能へ依存させないでください。両対応ならDMG/CGBそれぞれの描画と動作を確認してください。
- 英数字・記号には提供された自作 `ascii.c` 由来のフォントを使い、使用するコードとタイルの対応を確認してください。画像・音楽・効果音は編集可能な元データと生成手順も保存してください。
- ソースのコメントは英語、作業報告は日本語で記述してください。SARAKURAの標準レポートは英語のまま利用してください。

###### 3. 各反復でビルドと実行を結び付ける

`game-gb/out/iter-001` のように反復ごとの出力先を作り、実行コマンド、コンパイラの終了コード、ROM・メタデータ・ソース・ツールのハッシュを記録してください。
失敗したビルドの後に、残っている別のROMを実行しないでください。ROMと `.map`、ソースマップ、デバッグ情報は同じビルドのものを使ってください。

以下は親フォルダーから実行する基本形です。`main.c`、追加のライブラリ、オプション、入力列は実装と試験に合わせて設定してください。

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

`--hardware dmg` は初代GBの試験用です。両対応・CGB専用の試験では、ROMヘッダーの機種指定とKOKURAの機種指定も合わせてください。300フレームは動作確認の例で、ゲーム全体の試験時間を意味しません。

###### 4. 実際に操作し、画面・音・状態を照合する

- 入力シナリオをファイルに保存し、押下、保持、解放を区別してください。起動、ゲーム開始、移動、アクション、衝突、ステージ遷移、ゲームオーバー、再開、ポーズなど、仕様にある経路を通してください。
- 必要なフレームのPNG、入力列、実行レポート、診断JSONL、WAV、必要に応じた状態・メモリ観測を保存してください。実行フレーム数と停止理由も確認してください。
- 画面は画像として確認してください。カウンターや座標などは期待値とも照合してください。1枚の画面で動きや入力への反応を確認したことにしないでください。
- BGMや効果音の再生、同時発音、途切れ、ポーズ・再開を確認してください。音声ファイルがあることだけで音が正しいと判断しないでください。試聴できない環境では、その制約と実施できた波形・数値検査を分けて報告してください。
- 性能は重い場面でも測定してください。ホストPC上のエミュレータの実行速度を、ゲーム内の更新頻度や実機での速度と同一視しないでください。

###### 5. SARAKURAを使って原因を絞り、同条件で再検証する

- `--metadata` にはそのROMを作ったビルド情報、`--events` にはその実行から得た診断JSONLを渡してください。CPUトレースや通常の実行レポートを診断JSONLの代わりにしないでください。
- `report.html`、`ai_diagnostics.json`、`repair_prompt.md`、`retest_plan.json` を読み、診断を再現手順・画面・音・該当ソースと照合してください。SARAKURAはROMを実行したり、ソースを自動修正したりするツールではありません。
- 正常な待機ループと停止不具合を区別し、警告は個別に理由を判断してください。フィルターで非表示にしたり、フレーム数を減らしたりして合格扱いにしないでください。ソース位置や原因の推定は、確認済みの事実と区別してください。
- 不具合を最小化し、原因に対応する変更を加え、ビルドからやり直してください。ツール側の不具合が疑われる場合は、ゲーム側の問題と切り分ける最小再現例を作り、ツールの修正には回帰検証も付けてください。
- 修正前後で入力・乱数種・機種・観測フレーム・診断条件を揃え、同じ受け入れ条件を再実行してください。ROMが変わった場合はそのビルドに対応するメタデータを使い、状態ファイルを無条件に使い回さないでください。

```powershell
& '.\sarakura\sarakura.exe' baseline-delta `
  --baseline '.\game-gb\out\iter-001\analysis' `
  --current '.\game-gb\out\iter-002\analysis' `
  --out '.\game-gb\out\delta.json' --markdown '.\game-gb\out\delta.md' `
  --fail-on-new error --fail-on-regression error --enforce
```

差分診断は、操作・表示・音の合格判定と併用してください。同じ失敗を繰り返す場合はログと仮説を見直し、根拠なく試行を続けないでください。

###### 6. 完了条件と納品

完成版のROMと同じソース・設定で、すべての必須シナリオを再実行してください。検証用の無敵状態や自動入力だけで通常プレイを検証済みにしないでください。
必須要件と試験の対応表、残る警告の理由、未確認事項を明示してください。実機で試していない場合は「実機未確認」と記載してください。

納品物は、ソース、使用ライブラリとツールの識別情報、編集可能な素材、再現可能なビルド・検証スクリプト、ROM、最終検証の証拠、起動方法・操作・既知の制限を記したREADMEです。
公開・外部送信は明示された範囲で行ってください。不要な中間ビルドや一時トレースは、その反復の確認後に削除してください。ただし、ソース、素材、最終成果物、必要な回帰証拠を削除しないでください。

実行環境や権限などの障害で必須検証を実施できない場合は、完了とせず、再現手順と必要な対応を具体的に報告してください。

</details>
<!-- development-prompt:ja:end -->

### 商標と提携関係について

KITAQGBは自作ソフトウェア開発のための独立したオープンソースプロジェクトです。

任天堂との提携関係になく、同社の推奨、後援、承認を受けたものではありません。Game BoyおよびGame Boy Colorは任天堂の商標です。

適法に利用できる権利がない限り、任天堂のロゴ、公式パッケージ画像、公式フォント、BIOS、市販ROMのデータ、権利者が専有するゲーム素材をこのリポジトリへ追加しないでください。

### ライセンス

KITAQGBはMITライセンスで配布しています。

派生元であるNORCALの原著作権表示は次のとおりです。

```text
Copyright 2019 Keith Holman
```

KITAQGBの変更・追加部分の著作権表示は次のとおりです。

```text
Copyright (c) 2026 DAISUKE OBA
```

ソフトウェアの複製または重要な部分には、NORCALの原著作権表示とMITライセンスの表示を保持する必要があります。詳細は `LICENSE` と `THIRD_PARTY_NOTICES.md` を参照してください。

### 変更の提案について

変更を提出する際は、次の方針に従ってください。

- 著作権で保護されたROM、BIOS、市販作品から抽出した素材、公式SDKの資料を追加しないでください。
- リリース上の明確な理由がある場合を除き、`bin/`、`obj/`、`target/`、`dist/`、`*.exe`、`*.dll`、`*.pdb` などの生成物をソースのコミットに含めないでください。
- コンパイラやコード生成の不具合には、小さく再現可能なテストケースを用意してください。
- 支援ライブラリを追加する場合は、コンパイルコマンドと必要なハードウェアレジスタ宣言を記載してください。
- 人間とAIコーディングエージェントの双方が次の行動を判断できる診断情報にしてください。

### 現在の状況

このリポジトリはKITAQGBの初回公開版として整備しています。プロジェクトの成熟に伴い、インターフェース、支援ライブラリ、診断機能、関連ツールとの連携は変わる場合があります。

### ビルドと初回利用

Windows、.NET Framework 4.8 Developer Pack、Visual Studio Build Tools（MSBuild）を用意し、Developer PowerShellから実行します。

```powershell
MSBuild.exe .\kitaqgb\kitaqgb.csproj /t:Build /p:Configuration=Release
.\kitaqgb.exe --help
.\examples\build.ps1
```

### マニュアルとライセンス

- [日本語HTMLマニュアル](https://bartaro.github.io/kitaq-docs/kitaqgb.html) / [英語HTMLマニュアル](https://bartaro.github.io/kitaq-docs/en/kitaqgb.html)
- [オフライン用マニュアルのソース](https://github.com/bartaro/kitaq-docs)
- [ライセンス英語原文](LICENSE) / [日本語参考訳](LICENSE.ja)

プロジェクトのライセンスは、第三者のフォント、依存ライブラリ、ロゴ、商標に関する条件を置き換えるものではありません。再配布時は付属の権利表記も保持してください。

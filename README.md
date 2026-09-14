# KITAQGB

<!-- manual-language-links:start -->
| Language / 言語 | HTML |
| --- | --- |
| English | [KITAQGB](https://bartaro.github.io/kitaq-docs/en/kitaqgb.html) · [KITAQGB Library](https://bartaro.github.io/kitaq-docs/en/gb-library.html) |
| 日本語 | [KITAQGB](https://bartaro.github.io/kitaq-docs/kitaqgb.html) · [KITAQGB Library](https://bartaro.github.io/kitaq-docs/gb-library.html) |
| 한국어 | [KITAQGB](https://bartaro.github.io/kitaq-docs/ko/kitaqgb.html) · [KITAQGB Library](https://bartaro.github.io/kitaq-docs/ko/gb-library.html) |
| 简体中文 | [KITAQGB](https://bartaro.github.io/kitaq-docs/zh-CN/kitaqgb.html) · [KITAQGB Library](https://bartaro.github.io/kitaq-docs/zh-CN/gb-library.html) |
| 繁體中文 | [KITAQGB](https://bartaro.github.io/kitaq-docs/zh-TW/kitaqgb.html) · [KITAQGB Library](https://bartaro.github.io/kitaq-docs/zh-TW/gb-library.html) |
| Español | [KITAQGB](https://bartaro.github.io/kitaq-docs/es/kitaqgb.html) · [KITAQGB Library](https://bartaro.github.io/kitaq-docs/es/gb-library.html) |
| Português (Brasil) | [KITAQGB](https://bartaro.github.io/kitaq-docs/pt/kitaqgb.html) · [KITAQGB Library](https://bartaro.github.io/kitaq-docs/pt/gb-library.html) |
| Français | [KITAQGB](https://bartaro.github.io/kitaq-docs/fr/kitaqgb.html) · [KITAQGB Library](https://bartaro.github.io/kitaq-docs/fr/gb-library.html) |
| Deutsch | [KITAQGB](https://bartaro.github.io/kitaq-docs/de/kitaqgb.html) · [KITAQGB Library](https://bartaro.github.io/kitaq-docs/de/gb-library.html) |
<!-- manual-language-links:end -->


[English](#english) | [日本語](#japanese) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Español](README.es.md) | [Português (Brasil)](README.pt-BR.md) | [Français](README.fr.md) | [Deutsch](README.de.md)

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
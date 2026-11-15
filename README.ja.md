# KITAQGB

[English](README.en.md) | **日本語** | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Français](README.fr.md) | [Español](README.es.md) | [Português (Brasil)](README.pt-BR.md) | [Deutsch](README.de.md)


## KITAQGBとは

KITAQGBは、Game Boy系の自作ソフトウェア向けCコンパイラと支援ライブラリです。NORCALを基礎に、AIを活用した現代的なゲーム制作のための開発手順とツール連携を拡張しています。

主な対象は次のとおりです。

- CソースからGame Boy ROMへのコンパイル
- Game Boy / Game Boy Color向けの自作ソフトウェア開発
- AIが扱いやすい診断情報と再現可能なビルドレポート
- ROM・ヘッダー生成とLR35902系ターゲット向けの低水準コード生成
- Game Boy Color用のパレット、タイル、スクロール、カメラ、音声、通信、RPG・ADV・SLG支援ライブラリ
- KOKURA CLIと連携したエミュレータ上のテスト、トレース、デバッグ

市販ROM、任天堂のBIOS・SDK、権利者が専有する素材、任天堂の公式開発資料は同梱していません。

## 対象プラットフォーム

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

## 導入

リポジトリのZIPをダウンロードして展開してください。同梱のコンパイラ・補助ツールの実行にRust、.NET、Python、Pillowは不要です。

Windows用実行ファイルはリポジトリ直下、Linux・macOS用は下表のbin/配下にあります。lib/とライセンス表記を一緒に保持してください。Linux/macOSでは取得した実行ファイルにchmod +xで実行権限を付け、配置先をPATHに追加するかフルパスで実行します。

| OS | kitaqgb |
| --- | --- |
| Windows x64 | `./kitaqgb.exe` |
| Linux x64 (static musl) | `bin/linux-x86_64/kitaqgb` |
| Linux x64 (GNU) | `bin/linux-gnu-x86_64/kitaqgb` |
| macOS ARM64 | `bin/macos-arm64/kitaqgb` |
| macOS Intel | `bin/macos-x86_64/kitaqgb` |
| macOS Universal | `bin/macos-universal/kitaqgb` |

## 最初のプログラム

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

## 同梱ライブラリの利用

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

### ソースからビルド

Rust 1.85以降で、Windows・Linux・macOS ARM・macOS Intel用のコンパイラと全補助ツールをビルドできます。ネイティブ実行ファイルの動作に.NETは不要です。素材処理用の補助ツールもPythonやPillowを必要としません。

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

[Rust製コンパイラと補助ツール](tools/README.ja.md)

## 3D描画

Wire3Dの描画時間・88行プロファイル・独立した更新時計

DMGはWIRE3D_DMG_HEIGHTを88・96・120から選択できます。CGBはWIRE3DCGB_HEIGHTを88・96から選択します。既定値はDMG 120、CGB 96です。88行は128×88、中心Y=44です。ライブラリと呼び出し側で同じ設定を使い、wire3d_dmg_88.c / wire3d_cgb_88.cを通常のエントリの代わりにコンパイルします。DMGの88行は96行と同じモデル構造・16辺制限です。CGBの160×144モードは変わりません。

[描画の使い方と実行例](https://bartaro.github.io/kitaq-docs/gb-library.html#wire3d-feedback-20261009)

## よく使うコマンドラインオプション

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

## KOKURA CLIと連携する開発手順

KITAQGBは、エミュレータ・デバッガであるKOKURA CLIと組み合わせて使うことを想定しています。

```text
1. Cでゲームコードを書く、または生成する
2. KITAQGBでコンパイルする
3. 生成したROMをKOKURA CLIで実行する
4. 診断、トレース、シンボル、タイミング観測、エミュレータのレポートを取得する
5. 結果を次のコード修正・デバッグに反映する
```

コンパイルエラー、エミュレータのレポート、実行時トレースを具体的な修正作業に変えられるため、AIを活用した開発にも適しています。

## リポジトリの構成

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

## 開発方針

KITAQGBは汎用の現代的なCコンパイラを目指すものではありません。小規模で、タイミング制約とバンク構成を持つ8ビットゲーム機の自作開発に特化しています。

次の点を重視しています。

- 動作を予測しやすい生成コード
- 明確な診断情報
- 小さく再現可能なサンプル
- AIが読めるビルド・デバッグレポート
- 必要に応じた低水準の制御
- 利用しやすい高水準の支援ライブラリ

マシンの仕組みを完全に隠すことなく、Game Boy系の開発に取り組みやすくすることを目指しています。

## ゲーム開発プロンプト

[マニュアル](https://bartaro.github.io/kitaq-docs/loop-engineering.html#gb)

## 名称の由来

KITAQGBはZachtronicsに関連するNES用CコンパイラNORCALを出発点とするフォークです。原作者Keith Holman氏の著作権表示を保持しています。

NORCALの名前は北カリフォルニア（Northern California）に由来します。地域の名前を使うこの命名に着想を得て、作者は自身が生まれ育った北九州市をもとにKITAQGBと名付けました。KITAQ + GB、つまり日本の福岡県北九州市の愛称である北九（キタキュー、Kitakyū）とGame Boyを組み合わせた意味です。KITAQは「キタキュー」と読みます。英語話者向けの発音の目安は kee-tah-KYOO、発音記号は /ˌkiːtɑːˈkjuː/ です。最後のQは英語のアルファベットQと同じ音です。KITAQGBはGとBを一文字ずつ読み、「キタキュー・ジー・ビー」と呼びます。

KITAQGBという名前には、二つの意味があります。**Kernel-Informed Toolchain for AI-Quality Game Boy Development**は、対象マシンを理解し、人間のプログラマーと生成AIの双方を支えるツールチェーンという目標を表しています。

もう一つの意味は、**Kids' Imagination Transformed into Actual Quests in Game Boy Forests**です。「子どもの想像を、Game Boyの森で本物の冒険へ変える道具」という意味を込めています。小さなアイデアや落書き、AI支援による試作を、実際に遊べる冒険へ変えていくための道具にしたい、という創作面での願いを表しています。

## 変更の提案について

変更を提出する際は、次の方針に従ってください。

- 著作権で保護されたROM、BIOS、市販作品から抽出した素材、公式SDKの資料を追加しないでください。
- リリース上の明確な理由がある場合を除き、`bin/`、`obj/`、`target/`、`dist/`、`*.exe`、`*.dll`、`*.pdb` などの生成物をソースのコミットに含めないでください。
- コンパイラやコード生成の不具合には、小さく再現可能なテストケースを用意してください。
- 支援ライブラリを追加する場合は、コンパイルコマンドと必要なハードウェアレジスタ宣言を記載してください。
- 人間とAIコーディングエージェントの双方が次の行動を判断できる診断情報にしてください。

## 公開状況：パブリックプレビュー

KITAQGBとKOKURAは、現在パブリックプレビューの開発ツールとして公開しています。

実験、サンプル制作、AIを活用したゲーム開発、コンパイラの研究、エミュレータ上のデバッグ、開発手順の検証に利用できます。開発は継続中で、API、CLIオプション、出力形式、診断内容、動作はバージョンによって変わる場合があります。

プレビュー版には不具合、未完成の機能、互換性のない変更が含まれる可能性があります。製品や公開作品に使う前に、生成コード、エミュレータの動作、タイミング診断、レポートを十分に検証してください。

**Kernel-Informed Toolchain for AI-Quality Game Boy Development**

KITAQGBは、Game BoyおよびGame Boy Color向けの自作ソフトウェアを開発するオープンソースのCツールチェーンです。AIによる開発支援と診断情報の活用を想定しています。小さなCプログラムを書き、`.gb` / `.gbc` ROMへコンパイルし、エミュレータから得た情報を使ってゲームを改善できます。

KITAQGBは任天堂との提携関係になく、同社の推奨、後援、承認を受けたものではありません。Game BoyおよびGame Boy Colorは任天堂の商標です。

## マニュアルとライセンス

- [日本語HTMLマニュアル](https://bartaro.github.io/kitaq-docs/kitaqgb.html) / [英語HTMLマニュアル](https://bartaro.github.io/kitaq-docs/en/kitaqgb.html)
- [オフライン用マニュアルのソース](https://github.com/bartaro/kitaq-docs)
- [ライセンス英語原文](LICENSE) / [日本語参考訳](LICENSE.ja)

プロジェクトのライセンスは、第三者のフォント、依存ライブラリ、ロゴ、商標に関する条件を置き換えるものではありません。再配布時は付属の権利表記も保持してください。

## 商標と提携関係について

KITAQGBは自作ソフトウェア開発のための独立したオープンソースプロジェクトです。

任天堂との提携関係になく、同社の推奨、後援、承認を受けたものではありません。Game BoyおよびGame Boy Colorは任天堂の商標です。

適法に利用できる権利がない限り、任天堂のロゴ、公式パッケージ画像、公式フォント、BIOS、市販ROMのデータ、権利者が専有するゲーム素材をこのリポジトリへ追加しないでください。

## ライセンス

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

[マニュアル](https://bartaro.github.io/kitaq-docs/kitaqgb.html)

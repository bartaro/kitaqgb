<a name="japanese"></a>

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
## Rust製コンパイラと補助ツール

Rust 1.85以降で、Windows・Linux・macOS ARM・macOS Intel用のコンパイラと全補助ツールをビルドできます。ネイティブ実行ファイルの動作に.NETは不要です。素材処理用の補助ツールもPythonやPillowを必要としません。

```sh
cargo test --locked --tests
cargo build --locked --release
```

Windows用実行ファイルはリポジトリ直下、Linux・macOS用は下表のbin/配下にあります。lib/とライセンス表記を一緒に保持してください。Linux/macOSでは取得した実行ファイルにchmod +xで実行権限を付け、配置先をPATHに追加するかフルパスで実行します。

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


Windows・Linux・macOS ARM・macOS Intelでネイティブビルドと実行検証が成功しています。KITAQGBは各環境48件のテストと395件の補助ツール検証、KITAQFCは55件と401件が成功しました。Rust 1.85でも確認済みです。PUBLIC_DISTRIBUTION.jsonに配置済みバイナリのハッシュと検証の出典を記録しています。公開GitHub Actionsでもこのソースを独立してビルド・検証します。

保存済みの比較データでROMのバイト列、診断と補助ツールの形式を検証しています。以前のC#版によるエミュレータ検証は、そのソース指紋に対応する過去の記録として残しています。これだけでRust版の全API、実機動作、FDSのBIOS経由のゲーム起動を保証するものではありません。元のPNG変換スクリプトは入手できないため仕様から再実装し、元スクリプトとのバイト一致は確認できません。

[Rust製コンパイラと補助ツール](tools/README.ja.md)

<!-- rust-native-20261004:end -->

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

Rust 1.85以降で、Windows・Linux・macOS ARM・macOS Intel用のコンパイラと全補助ツールをビルドできます。ネイティブ実行ファイルの動作に.NETは不要です。素材処理用の補助ツールもPythonやPillowを必要としません。

```sh
cargo test --locked --tests
cargo build --locked --release
```

Windows用実行ファイルはリポジトリ直下、Linux・macOS用は下表のbin/配下にあります。lib/とライセンス表記を一緒に保持してください。Linux/macOSでは取得した実行ファイルにchmod +xで実行権限を付け、配置先をPATHに追加するかフルパスで実行します。

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

### 必要な環境

Rust 1.85以降で、Windows・Linux・macOS ARM・macOS Intel用のコンパイラと全補助ツールをビルドできます。ネイティブ実行ファイルの動作に.NETは不要です。素材処理用の補助ツールもPythonやPillowを必要としません。

```sh
cargo test --locked --tests
cargo build --locked --release
```

Windows用実行ファイルはリポジトリ直下、Linux・macOS用は下表のbin/配下にあります。lib/とライセンス表記を一緒に保持してください。Linux/macOSでは取得した実行ファイルにchmod +xで実行権限を付け、配置先をPATHに追加するかフルパスで実行します。

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

### KITAQGBのビルド

Rust 1.85以降で、Windows・Linux・macOS ARM・macOS Intel用のコンパイラと全補助ツールをビルドできます。ネイティブ実行ファイルの動作に.NETは不要です。素材処理用の補助ツールもPythonやPillowを必要としません。

```sh
cargo test --locked --tests
cargo build --locked --release
```

Windows用実行ファイルはリポジトリ直下、Linux・macOS用は下表のbin/配下にあります。lib/とライセンス表記を一緒に保持してください。Linux/macOSでは取得した実行ファイルにchmod +xで実行権限を付け、配置先をPATHに追加するかフルパスで実行します。

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

Rust 1.85以降で、Windows・Linux・macOS ARM・macOS Intel用のコンパイラと全補助ツールをビルドできます。ネイティブ実行ファイルの動作に.NETは不要です。素材処理用の補助ツールもPythonやPillowを必要としません。

```sh
cargo test --locked --tests
cargo build --locked --release
```

Windows用実行ファイルはリポジトリ直下、Linux・macOS用は下表のbin/配下にあります。lib/とライセンス表記を一緒に保持してください。Linux/macOSでは取得した実行ファイルにchmod +xで実行権限を付け、配置先をPATHに追加するかフルパスで実行します。

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

### マニュアルとライセンス

- [日本語HTMLマニュアル](https://bartaro.github.io/kitaq-docs/kitaqgb.html) / [英語HTMLマニュアル](https://bartaro.github.io/kitaq-docs/en/kitaqgb.html)
- [オフライン用マニュアルのソース](https://github.com/bartaro/kitaq-docs)
- [ライセンス英語原文](LICENSE) / [日本語参考訳](LICENSE.ja)

プロジェクトのライセンスは、第三者のフォント、依存ライブラリ、ロゴ、商標に関する条件を置き換えるものではありません。再配布時は付属の権利表記も保持してください。

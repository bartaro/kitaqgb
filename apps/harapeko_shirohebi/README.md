# HARAPEKO SHIROHEBI

[English](#english) · [日本語](#日本語)

## English

A Game Boy / Game Boy Color score-attack game by **DAISUKE OBA**. Steer the white snake, eat pomegranates to grow, and avoid mines and your own body.

- **Play / official ROM download:** <https://bartaro.itch.io/harapeko-shirohebi>
- **Program guide:** [English HTML](docs/guide-en.html) · [日本語HTML](docs/guide-ja.html)
- **License:** [MIT](LICENSE), copyright © 2026 DAISUKE OBA. This covers the game sources, supplied original graphics, font data, music, sound effects and documentation in this directory. Keep the license notice when redistributing them. KITAQGB and its dependencies retain the notices in the [repository license](../../LICENSE), including the [original ASCII font notice](../../licenses/fonts/ASCII-font-MIT.txt).

The HTML guides contain a program flowchart, the snake-following algorithm, library examples and a source-file guide. Download the repository and open either HTML file in a browser; no server or internet connection is needed. GitHub's file viewer displays HTML as source.

### Build on Windows

You need Windows, .NET Framework 4.8, PowerShell, and a complete checkout or ZIP of this repository. Keep the root compiler files and `lib/` together. The exported graphics and audio are supplied as C arrays; no asset editor is needed.

From the repository root:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\apps\harapeko_shirohebi\build.ps1
```

Or, from this game's directory:

```powershell
.\build.ps1
# If the game is stored separately, select a complete KITAQGB installation:
.\build.ps1 -KitaqgbRoot C:\tools\kitaqgb
# Retain compiler intermediates for debugging:
.\build.ps1 -KeepBuildFiles
```

The default output is `out/shirohebi.gb`, with `out/shirohebi.map` and `out/build_manifest.json`. Use `-OutputDirectory C:\build\shirohebi` to choose another destination. Successful builds remove their temporary compilation directory unless `-KeepBuildFiles` is set; failed builds retain it for diagnosis. Output files are ignored by Git.

The script assembles the split game sources, compiles against this repository's libraries, verifies interrupt routines are in fixed ROM, installs the VBlank vector, and updates cartridge checksums. It produces a **64 KiB MBC5 ROM with 8 KiB battery-backed RAM**, playable in DMG and CGB modes. Do not compile individual split files independently or omit the vector/checksum step.

This application directory contains **no prebuilt executable or ROM**. The compiler is provided at the repository root; download the released game ROM from itch.io.

### Controls

| Screen | Operation |
|---|---|
| Title | START: begin. UP/DOWN/SELECT: select MUSIC or SOUND. LEFT/RIGHT/A: toggle the selected option. |
| Play | LEFT/RIGHT: turn relative to the snake's heading. Hold UP: accelerate. START: pause. |
| Pause | START: resume. SELECT: open the return-to-title confirmation. |
| Confirmation | LEFT/RIGHT/SELECT: choose. A: confirm. B/START: cancel. |
| Name entry | UP/DOWN: choose A–Z or a period. LEFT/RIGHT/SELECT: move the cursor. A: advance, or finish on the third character. START: finish. |
| Retry | LEFT/RIGHT/SELECT: choose YES/NO. A/START: confirm. |

On the title, SELECT+START opens the score-erasure confirmation. Screen transitions require all buttons to be released before accepting a fresh command. See the HTML guide for the program's state transitions and implementation details.

## 日本語

**DAISUKE OBA**制作のゲームボーイ／ゲームボーイカラー用スコアアタックゲームです。白ヘビを操り、ザクロを食べて体を伸ばしながら、地雷と自分の胴体を避けます。

- **ゲーム紹介・公開ROMのダウンロード:** <https://bartaro.itch.io/harapeko-shirohebi>
- **プログラム解説:** [日本語HTML](docs/guide-ja.html) · [English HTML](docs/guide-en.html)
- **ライセンス:** [MIT](LICENSE)（[日本語訳](LICENSE.ja)）、著作権者はDAISUKE OBA、年は2026年です。このディレクトリのゲームソース、付属の自作画像・フォントデータ・音楽・効果音・解説を対象とします。再配布時にはライセンス表示を保持してください。KITAQGBと依存物については[リポジトリのLICENSE](../../LICENSE)および[元のASCIIフォントの表示](../../licenses/fonts/ASCII-font-MIT.txt)が適用されます。

HTML解説にはフローチャート、白ヘビの追従アルゴリズム、ライブラリの利用例、ソースファイル一覧を収録しています。リポジトリをダウンロードしてHTMLをブラウザで開けば、サーバーやインターネット接続なしで読めます。GitHubのファイル表示ではHTMLのソースが表示されます。

### Windowsでのビルド

Windows、.NET Framework 4.8、PowerShellと、このリポジトリ一式を用意してください。直下のコンパイラ関連ファイルと`lib/`は一緒に置きます。画像・音声はC配列として付属しているため、素材エディタは不要です。

リポジトリ直下から実行します。

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\apps\harapeko_shirohebi\build.ps1
```

ゲームのディレクトリで実行する場合は次のとおりです。

```powershell
.\build.ps1
# ゲームを別の場所に置いた場合は、KITAQGB一式の場所を指定します。
.\build.ps1 -KitaqgbRoot C:\tools\kitaqgb
# 調査用にコンパイラの中間ファイルを残します。
.\build.ps1 -KeepBuildFiles
```

既定の出力先は`out/shirohebi.gb`です。シンボルマップ`out/shirohebi.map`とビルド情報`out/build_manifest.json`も生成します。`-OutputDirectory C:\build\shirohebi`で出力先を変更できます。成功時は一時ビルドディレクトリを削除し、`-KeepBuildFiles`指定時と失敗時は調査用に残します。生成物はGitの管理対象から除外します。

スクリプトは分割ソースを結合し、リポジトリのライブラリとコンパイルして、割り込み処理の固定バンク配置を確認します。その後VBlankベクタとカートリッジのチェックサムを設定します。出力はDMG・CGB両対応の**64 KiB MBC5 ROM、8 KiBバッテリーバックアップRAM**です。分割ソースを個別にコンパイルしたり、ベクタ・チェックサム処理を省略したりしないでください。

このアプリのディレクトリには**実行ファイルとROMを収録していません**。コンパイラはリポジトリ直下、公開済みのゲームROMはitch.ioから入手できます。

### 操作

| 画面 | 操作 |
|---|---|
| タイトル | STARTで開始。上下／SELECTでMUSIC・SOUNDを選択し、左右／Aで切り替えます。 |
| プレイ中 | 左右で進行方向に対して旋回。上を押して加速。STARTで一時停止。 |
| 一時停止中 | STARTで再開。SELECTでタイトルに戻る確認を開きます。 |
| 確認画面 | 左右／SELECTで選択、Aで確定、B／STARTで取消。 |
| 名前入力 | 上下でA〜Z・ピリオドを選択。左右／SELECTで位置移動。Aで次の文字へ進み、3文字目で確定。STARTでも確定。 |
| リトライ | 左右／SELECTでYES・NOを選び、A／STARTで確定。 |

タイトルでSELECT+STARTを押すと、スコア消去の確認画面が開きます。画面の切り替え後は、一度すべてのボタンを離してから操作します。内部の状態遷移や処理の詳細はHTML解説を参照してください。

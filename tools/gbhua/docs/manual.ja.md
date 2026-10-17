# GBHUA

## ダウンロードと起動

[GUIダウンロード](../GUI.md)でOSに合うアーカイブを選び、全体を展開してください。Windowsは`GBHUA.exe`を開き、隣の`assets`フォルダも残します。Macは`GBHUA.app`をアプリケーションフォルダへコピーして開きます。Apple Silicon版とIntel版を間違えないでください。配布GUIの実行にRustやPythonは不要です。Developer ID署名・公証は未実施です。公式リポジトリから入手し、Macに拒否された場合は出所を確認した上で「システム設定 > プライバシーとセキュリティ」からこのアプリだけを許可してください。OS全体の保護は無効にしないでください。

GUI内のラベルは英語です。この説明書が日本語の操作ガイドです。Macでも`Ctrl`はControlキーを指します。以下のソースビルド手順はリポジトリのルートから開始します。CLIは引き続きGUI不要で利用できます。

[English](manual.en.md) | [日本語](manual.ja.md) | [한국어](manual.ko.md) | [简体中文](manual.zh-CN.md) | [繁體中文](manual.zh-TW.md) | [Español](manual.es.md) | [Português](manual.pt.md) | [Français](manual.fr.md) | [Deutsch](manual.de.md)

GBHUAはPNG画像からGame Boy用のタイル・パレット・マップを作るツールです。作者はDAISUKE OBA、独自コードのライセンスはMITです。

## ビルドと起動

```powershell
cd tools/gbhua
cargo run --release --locked --features gui --bin gbhua_gui
```

## GUI

起動直後の未編集状態では未保存の警告は出ません。データを編集した後の確認は残ります。Windows版GUIは標準でログ用コンソールを開きません。診断時のみ `cargo run -p gbhua --bin gbhua_gui --features gui-console` でコンソール付き起動ができます。

各欄の見出しとキャンバスの上端を統一しました。上部のフォルダー・保存・Undo/Redoアイコンにマウスを重ねると名称が表示されます。Graphic上のサイズボタンで寸法を選び、Map上の全体表示・マイナス・プラスで倍率を変えます。DMG/CGBは右上で切り替えます。幅1180px未満では左欄のTiles / Palettesタブを切り替えます。パレットと診断情報はホイールまたはスクロールバーで移動できます。Escapeでメニューを閉じます。

1. 左の一覧でタイルを選び、Graphic > 8x8で8x8、16x16、32x32、8x16、16x8、16x32、32x16を指定します。大きな絵は選択IDから連続したタイルを使います。色を選んでドラッグで描き、右クリックで色を取得します。

2. TOOLS > FILLは同色の連結領域、TILES > FILL TILEは絵全体を塗ります。FLIP X/YとROTATEで変形し、EDIT > COPY GRAPHIC / PASTE GRAPHICで複製します。UNDO/REDOは最大64操作です。

3. TOOLS > MAPでマップに配置し、右クリックでタイルとパレットを取得します。RECTは3x3配置、FILLは連結領域、ATTRはパレット設定とX反転、COLL/EVTは印の切り替えです。VIEWでFit/1x/2x/4x、グリッドと印の表示を選び、ホイールやスクロールバーで移動します。

4. BYTECODEをクリックし、Ctrl+A、Ctrl+Vで16バイトを貼り付けます。IMPORTで先頭の8x8タイルに適用し、EXPORTでタイルを文字列に戻します。矢印/Home/Endで移動、Shiftで選択、Ctrl+C/Xでコピー・切り取り、Backspace/Deleteで前後の文字を削除します。

5. FILE > SAVEは毎回名前を尋ねます。EXPORTでもC、PNG、GBR、GBMの保存先を指定します。未保存の*があると読み込み・取り込み・終了時に確認します。Yesは保存、Noは破棄、Cancelは編集継続です。PNG取り込みはプロジェクトを置き換え、Undoで戻せます。

6. SPRITES > NEW METAでメタスプライトを作ります。ADD FRAME/DUP FRAME/CLEAR FRAMEとPREV/NEXT FRAMEでフレームを管理し、METASPRITEではクリックで部品を置き、右クリックで削除します。DMG / CGBは保存済みパレットを変更せずDMG/CGBを比較します。

```text
0x78,0x78,0xCC,0xCC,0xCC,0xCC,0xC0,0xC0,
0xDC,0xDC,0xCC,0xCC,0x7C,0x7C,0x00,0x00
```

全情報の保存には.gbhを使います。旧JSONプロジェクトは拡張子を.gbhに変えるだけで移行できます。旧拡張子の読み込み機能はありません。GBTD/GBR/GBTBとGBMB/GBMは交換用で、全情報は保持しません。GBMB書き出しでは同名の.gbrも生成されるため、2ファイルを一緒に管理します。

## PNG / Python / CLI

画像の縦横は8の倍数（8～2040）です。--sizeは最近傍でサイズを変更します。DMGは4階調、CGBはRGB555相当・タイルごと4色・最大8パレットへの近似です。透明部分は白に合成します。同じタイルは共有し、256種類を超える場合は中止します。警告とプレビューを確認してください。

```powershell
python -m venv .venv
.\.venv\Scripts\python -m pip install maturin
.\.venv\Scripts\python -m maturin develop --features python
```

```python
import gbhua
project = gbhua.Project.from_image("artwork.png", mode="cgb")
project.save("scene.gbh")
project.save_preview("preview.png", "cgb", 2)
print(project.validate_json())
```

[CLI: 日本語](../README.ja.md)

## PNG取り込みと困ったとき

`FILE > IMPORT PNG`またはPNGのドラッグ＆ドロップで取り込みます。先にDMG/CGBを選んでください。現在のプロジェクトを置き換えるため、必要な編集内容は先に保存します。CGB自動パレット選択は色の誤差を評価し、少数のハイライトや暗い輪郭が頻出する暗色だけに置き換わらないようにします。ただしGame Boyの色数制限は残ります。以前の取り込みで消えた光沢は元PNGを再取り込みしてください。減色済みの`.gbh`だけでは失われた画素を復元できません。`--size`はCLIのオプションでありGUIの入力欄ではありません。

バイトコードが反映されない場合は、対象タイルを選択し、BYTECODEの内側をクリックして16バイトに置き換え、IMPORTを押します。EXPORTは文字列への変換のみです。マップの色や向きが違う場合はパレット・反転属性を確認してください。編集情報の保存には`.gbh`を選びます。エラーは下部ステータスに表示され、保存ダイアログのキャンセルでは保存されません。

macOS/LinuxのPython環境は`python3 -m venv .venv`、`source .venv/bin/activate`、`python -m pip install maturin`、`python -m maturin develop --features python`の順に作成します。バインディングのビルドにはPython 3.10以降とRustが必要です。

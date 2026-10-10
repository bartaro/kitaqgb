# GBHUA

[English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Español](README.es.md) | [Português](README.pt.md) | [Français](README.fr.md) | [Deutsch](README.de.md)

GBHUAはPNG画像からGame Boy用のタイル・パレット・マップを作るツールです。作者はDAISUKE OBA、独自コードのライセンスはMITです。

## ビルドと起動

```sh
cd tools/gbhua
cargo build --release --locked
./target/release/gbhua --help
```

Windows: `target\release\gbhua.exe`. Rust >= 1.92.

## macOS実行ファイル

Apple Silicon用・Intel Mac用のCLI配布版は[ダウンロードと起動手順](MACOS.md)を参照してください。
対応するアーカイブを展開し、ターミナルでそのフォルダに移動して`./gbhua --help`で起動します。
以下の例の`./target/release/gbhua`は`./gbhua`に置き換えてください。Rust・Pythonのインストールは不要です。
最低OS設定はmacOS 11.0、実行検証はmacOS 15です。Apple Developer ID署名・公証はしていません。

## 基本手順

PNGを取り込み、短いJSONレポートで確認し、検証、プレビュー、書き出しの順に進めます。成功時の出力はJSON一つです。AIにはプロジェクト全文ではなくファイルパスとレポートを渡すとトークン量を抑えられます。

```sh
./target/release/gbhua import-image artwork.png --out scene.gbh --mode cgb --size 160x144
./target/release/gbhua inspect scene.gbh
./target/release/gbhua validate scene.gbh
./target/release/gbhua preview scene.gbh --out preview.png --mode cgb --scale 2
./target/release/gbhua export scene.gbh --out scene.c --prefix scene
./target/release/gbhua export scene.gbh --out tiles.gbtb
./target/release/gbhua export scene.gbh --out world.gbmb
./target/release/gbhua export scene.gbh --out tiles.2bpp
```

画像の縦横は8の倍数（8～2040）です。--sizeは最近傍でサイズを変更します。DMGは4階調、CGBはRGB555相当・タイルごと4色・最大8パレットへの近似です。透明部分は白に合成します。同じタイルは共有し、256種類を超える場合は中止します。警告とプレビューを確認してください。

全情報の保存には.gbhを使います。旧JSONプロジェクトは拡張子を.gbhに変えるだけで移行できます。旧拡張子の読み込み機能はありません。GBTD/GBR/GBTBとGBMB/GBMは交換用で、全情報は保持しません。GBMB書き出しでは同名の.gbrも生成されるため、2ファイルを一緒に管理します。

既存ファイルの上書きには--forceが必要です。入力と出力には別のパスを指定します。終了コードは0=成功、1=データ検証不合格、2=引数・I/O・変換エラーです。画像生成APIやネットワーク送信機能はありません。依存関係の権利は各作者にあり、LICENSE、THIRD_PARTY_NOTICES.md、DEPENDENCIES.jsonを参照してください。

## GUI / Python

ローカルGUIとPython bindingsも同じ変換コアを使います。独立CLIパッケージにGUIは含まれません。ローカルのフル構成ではgbhua_guiを起動し、Pythonではimport gbhuaを使います。GUIはグループ別メニュー、保存先ダイアログ、DMG/CGB比較、7種類の描画サイズ、16バイトの入力、Undo/Redo、マップ拡大・スクロールに対応します。


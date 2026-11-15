# ネイティブ補助ツール

[English](README.en.md) | **日本語** | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Français](README.fr.md) | [Español](README.es.md) | [Português (Brasil)](README.pt.md) | [Deutsch](README.de.md)

ZX0/VBlank補助実行ファイルはRust実装です。実行時に.NET・Python・Pillowは不要です。`cargo build --locked --release`でビルドします。Windowsでは各コマンド名に`.exe`を付けます。


- [GBHUA](gbhua/README.ja.md)
- [GBSEQ](gbseq/README.ja.md)
- [マニュアル](https://bartaro.github.io/kitaq-docs/kitaqgb.html)

## GBHUA

[GUIダウンロード](gbhua/GUI.md) / [GUI日本語説明書](gbhua/docs/manual.ja.md) / [全9言語の説明書](gbhua/README.md)。
Windows版、Mac Apple Silicon版、Mac Intel版のGUIと、GUI・CLI共通のソース、Python bindingsを公開しています。
GUIは `cargo run --manifest-path tools/gbhua/Cargo.toml --locked --release --features gui --bin gbhua_gui` で起動できます。

[GBHUA CLI](gbhua/) はPNGからGB/DMG・CGB用タイル、パレット、マップを作成する独立ツールです。`.gbh`・GBTD/GBTB・GBMBの入出力と、C・2bppへの書き出しに対応します。作者: DAISUKE OBA。独自コード: MIT。GUI・GPU・Pythonは不要です。

[日本語説明書](gbhua/README.ja.md) / [9言語の説明書](gbhua/README.md)。
リポジトリのルートから `cargo build --manifest-path tools/gbhua/Cargo.toml --locked --release` で別途ビルドします。

## ZX0 / VBlank

```text
kitaqgb-patch-vblank --rom game.gb --map game.map
kitaqgb-zx0 input.bin output.zx0
kitaqgb-zx0 output.zx0 restored.bin --decompress
kitaqgb-zx0 input.bin asset.h --header=level_data
kitaqgb-zx0 input.bin output.kqa --format=auto
```

ZX0は1～65535バイトを受け付け、C#圧縮器の出力を保持します。`raw`、個数・値の`rle`、9バイトの`KQA1`自動選択コンテナに対応します。同サイズならraw、RLE、ZX0の順に選びます。`--decompress`は裸の順方向ZX0 v2ストリームを上限付きで展開します。逆方向ストリームとv1は対象外です。形式の設計者はEinar Saukas氏で、このKITAQ実装はMITライセンスです。

VBlank修正ツールは固定バンクのマップシンボルとPUSH命令の並びを確認し、指定ROMを直接書き換えます。ヘッダー・全体チェックサムも更新します。`--no-header-fix`ではチェックサムを更新しません。命令の検索は推定であり、割り込み処理全体の正しさを検証するものではありません。修正前にバックアップを保存してください。

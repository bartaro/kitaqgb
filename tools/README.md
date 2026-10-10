# ネイティブ補助ツール

[en](README.en.md) · [ja](README.ja.md) · [ko](README.ko.md) · [zh-CN](README.zh-CN.md) · [zh-TW](README.zh-TW.md) · [fr](README.fr.md) · [es](README.es.md) · [de](README.de.md)

ZX0/VBlank補助実行ファイルはRust実装です。実行時に.NET・Python・Pillowは不要です。`cargo build --locked --release`でビルドします。Windowsでは各コマンド名に`.exe`を付けます。

## GBHUA

[GBHUA CLI](gbhua/) はPNGからGB/DMG・CGB用タイル、パレット、マップを作成する独立ツールです。`.gbh`・GBTD/GBTB・GBMBの入出力と、C・2bppへの書き出しに対応します。作者: DAISUKE OBA。独自コード: MIT。GUI・GPU・Pythonは不要です。

[日本語説明書](gbhua/README.ja.md) / [9言語の説明書](gbhua/README.md)。
リポジトリのルートから `cargo build --manifest-path tools/gbhua/Cargo.toml --locked --release` で別途ビルドします。

## ZX0 / VBlank

```text
kitaqgb-zx0 input.bin output.zx0
kitaqgb-zx0 output.zx0 restored.bin --decompress
kitaqgb-zx0 input.bin asset.h --header=level_data
kitaqgb-zx0 input.bin output.kqa --format=auto
kitaqgb-patch-vblank --rom game.gb --map game.map
```

ZX0は1～65535バイトを受け付け、C#圧縮器の出力を保持します。`raw`、個数・値の`rle`、9バイトの`KQA1`自動選択コンテナに対応します。同サイズならraw、RLE、ZX0の順に選びます。`--decompress`は裸の順方向ZX0 v2ストリームを上限付きで展開します。逆方向ストリームとv1は対象外です。形式の設計者はEinar Saukas氏で、このKITAQ実装はMITライセンスです。

VBlank修正ツールは固定バンクのマップシンボルとPUSH命令の並びを確認し、指定ROMを直接書き換えます。ヘッダー・全体チェックサムも更新します。`--no-header-fix`ではチェックサムを更新しません。命令の検索は推定であり、割り込み処理全体の正しさを検証するものではありません。修正前にバックアップを保存してください。

<!-- wire3d-feedback:start -->

Wire3D timing, 88-row profiles and independent clocks

DMG accepts WIRE3D_DMG_HEIGHT 88, 96 or 120; CGB accepts WIRE3DCGB_HEIGHT 88 or 96. Defaults remain DMG 120 and CGB 96. The 88-row viewport is 128×88 with center Y=44. Use the same setting in the library and caller, compiling wire3d_dmg_88.c / wire3d_cgb_88.c instead of the normal entry. DMG 88 shares the 96-row model layout and 16-edge limit. CGB 160×144 mode is unchanged.

[Verification results and examples](https://bartaro.github.io/kitaq-docs/en/gb-library.html#wire3d-feedback-20261009)

Wire3D measurement and regression scripts use Python 3. Emulator checks require the KOKURA Python bridge and C API DLL. Pillow is optional for PNG output.

<!-- wire3d-feedback:end -->

# ネイティブ補助ツール

[en](README.en.md) · [ja](README.ja.md) · [ko](README.ko.md) · [zh-CN](README.zh-CN.md) · [zh-TW](README.zh-TW.md) · [fr](README.fr.md) · [es](README.es.md) · [de](README.de.md)

すべての補助実行ファイルはRust実装です。実行時に.NET・Python・Pillowは不要です。`cargo build --locked --release`で全実行ファイルをビルドします。Windowsでは各コマンド名に`.exe`を付けます。

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

Wire3Dの描画時間・88行プロファイル・独立した更新時計

DMGはWIRE3D_DMG_HEIGHTを88・96・120から選択できます。CGBはWIRE3DCGB_HEIGHTを88・96から選択します。既定値はDMG 120、CGB 96です。88行は128×88、中心Y=44です。ライブラリと呼び出し側で同じ設定を使い、wire3d_dmg_88.c / wire3d_cgb_88.cを通常のエントリの代わりにコンパイルします。DMGの88行は96行と同じモデル構造・16辺制限です。CGBの160×144モードは変わりません。

[検証結果と実行例](https://bartaro.github.io/kitaq-docs/gb-library.html#wire3d-feedback-20261009)

Wire3Dの計測・回帰検証スクリプトはPython 3を使います。エミュレータ検証にはKOKURAのPythonブリッジとC API DLLが必要です。PNG出力用のPillowは任意です。

<!-- wire3d-feedback:end -->

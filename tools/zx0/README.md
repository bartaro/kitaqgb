# ZX0-compatible asset compression

[English](#english) | [日本語](#日本語)

## English

The PC compressor and GB decompressor are independent KITAQ implementations
for ZX0 v2 forward streams. The KITAQ implementation is released under the
MIT License, copyright (c) 2026 DAISUKE OBA.

The ZX0 format and original compression algorithm were designed by
[Einar Saukas](https://github.com/einar-saukas/ZX0).
The implementation credit and license are separate from this format credit;
see the repository's LICENSE and LICENSE.ja.

Build the host tool from the repository root:

```powershell
.\tools\zx0\build.ps1
.\kitaqgb-zx0.exe input.bin output.zx0
.\kitaqgb-zx0.exe input.bin asset.h --header=level_data
.\kitaqgb-zx0.exe output.zx0 restored.bin --decompress
```

The standalone tool uses .NET Framework 4.x. Compression accepts 1–65535
input bytes. It uses a bounded hash-chain search and does not promise the
optimal compressed size. It produces ordinary ZX0 v2 data, without a
KITAQ wrapper. Backward streams, prefix dictionaries and ZX0 v1 are outside
this interface. Original asset rights remain with their authors.

All encoded outputs must fit the target API's 65535-byte size parameter.
Automatic containers include their nine-byte header in this limit. Split
larger assets, and also respect the target's much smaller RAM and bank
windows. An empty raw C header contains a placeholder storage byte and a
logical `_SIZE` of zero; an empty bare ZX0 stream is not supported.

To compare raw data, count/value RLE and ZX0 and keep the smallest payload:

```powershell
.\kitaqgb-zx0.exe input.bin output.kqa --format=auto
```

The automatic mode adds a nine-byte KQA1 header and reports the selected
codec. It compares payload sizes, with ties favoring raw, then RLE, then
ZX0. This is a KITAQ asset container, not a bare ZX0 stream. The header is
`KQA1`, codec byte (0 raw, 1 RLE, 2 ZX0), original size as u16 little-endian,
then payload size as u16 little-endian. The body follows immediately.
Use `asset_decompress` for this container. `--format=raw` and `--format=rle`
instead emit only the selected payload; the RLE terminator is a zero count.

Include `zx0.h` and compile `lib/zx0.c` for the target. `zx0_decompress`
takes destination, capacity, source and compressed byte count. Check both
the returned length and `zx0_error`. Errors can leave partial output, so
do not display or use it after a failed call. Buffers must not overlap,
wrap their CPU address range or cross currently mapped bank windows.
The decoder uses shared scratch and must not be re-entered from interrupts.

`zx0_decompress_vram` writes to the selected GB VRAM bank only while the
LCD is off. It preserves display, bank and interrupt settings. Upload
during scene initialization; do not assume a whole asset fits one VBlank.

## 日本語

PC側の圧縮器とGB側の展開器は、ZX0 v2の順方向ストリームに対応するKITAQの
独自実装です。KITAQ実装の著作権表示はCopyright (c) 2026 DAISUKE OBAで、
MITライセンスを適用します。

ZX0の圧縮形式と元の圧縮アルゴリズムの設計者は
[Einar Saukas氏](https://github.com/einar-saukas/ZX0)です。
形式への謝辞とKITAQ実装の著作権・ライセンスを分けて記載しています。
リポジトリのLICENSEとLICENSE.jaも参照してください。

上のPowerShellコマンドは、順にツールのビルド、ZX0ファイルの作成、
Cヘッダーの作成、PC上での展開を行います。実行には.NET Framework 4.xを使います。
入力は1～65535バイトで、探索回数を制限したハッシュチェーンで圧縮します。
最小サイズを保証する最適圧縮ではありません。逆方向展開、外部辞書、ZX0 v1は
対象外です。素材自体の権利は素材の作者に帰属します。

圧縮後の出力も65535バイト以内に収めます。自動選択形式では9バイトのヘッダーも
含めた上限です。さらに実機のRAM容量やバンク境界に合わせ、必要なら素材を分割して
ください。空のrawをCヘッダーへ出力すると保存領域として1バイトを用意しますが、
論理的な`_SIZE`は0です。空の裸のZX0ストリームは作成しません。

`--format=auto`では非圧縮、個数・値のRLE、ZX0のペイロードサイズを比べます。
同サイズなら非圧縮、RLE、ZX0の順に選び、9バイトのKQA1ヘッダーを付けます。
ヘッダーは識別子KQA1、方式1バイト、展開後サイズ2バイト、圧縮サイズ2バイトです。
2バイト値は下位バイトが先です。この形式には`asset_decompress`を使います。
`--format=raw`と`--format=rle`はヘッダーなしのデータを出力します。

ターゲット側では`zx0.h`を読み込み、`lib/zx0.c`もコンパイルしてください。
`zx0_decompress`には出力先、出力容量、入力元、圧縮バイト数を指定し、戻り値と
`zx0_error`を確認します。失敗時は途中まで書き込まれる場合があるため、出力を
使わないでください。入力と出力は重ねず、CPUアドレスの折り返しや現在のバンクの
境界をまたがないようにします。共有作業領域を使うため、割り込みから再入できません。

`zx0_decompress_vram`はLCD停止中に、選択中のGB VRAMバンクへ展開します。
表示・バンク・割り込みの設定は保持します。場面の初期化などで使い、素材全体の
展開が1回のVBlankに収まるとは仮定しないでください。

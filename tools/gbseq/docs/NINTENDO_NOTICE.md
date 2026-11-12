# Nintendo logo data in ROM headers / ROMヘッダーの任天堂ロゴについて

## English

GBSEQ's embedded LINK16 templates, and the ROMs exported from those templates,
include the 48-byte Nintendo logo data at cartridge-header offsets
`0x0104..0x0133` solely for compatibility with the console's boot-time validation.
These bytes are part of the compatible cartridge header; they are not a
Nintendo boot ROM, BIOS or SDK.

Including this data does not constitute a claim of ownership of Nintendo's
logo or trademarks. GBSEQ's MIT License does not grant an independent license
to any rights Nintendo may hold in that logo or those trademarks.

GBSEQ is an independent project and is not affiliated with, endorsed by,
sponsored by, or approved by Nintendo. Game Boy and Game Boy Color are
trademarks of Nintendo.

This notice clarifies the scope of the project's license. It adds no
restrictions to the MIT-licensed project code and does not replace any
third-party license. It is not a guarantee that a particular use or
distribution is legally permitted.

## 日本語

GBSEQに組み込まれたLINK16テンプレートと、それを用いて出力するROMには、
実機の起動時検証との互換性のため、カートリッジヘッダーの
`0x0104..0x0133` に48バイトの任天堂ロゴデータが含まれます。
これは互換カートリッジのヘッダーデータであり、任天堂のブートROM・BIOS・SDKではありません。

このデータの収録は、任天堂のロゴや商標に対する所有権を主張するものではありません。
GBSEQのMITライセンスは、それらについて任天堂が有する権利に関し、
独立した利用許諾を与えるものではありません。

GBSEQは独立したプロジェクトであり、任天堂との提携関係になく、
同社の推奨・後援・承認を受けたものではありません。
Game BoyおよびGame Boy Colorは任天堂の商標です。

この注意書きはプロジェクトのライセンスの適用範囲を明確にするもので、
MITライセンスの対象コードに追加の制限を課したり、第三者のライセンスを
置き換えたりするものではありません。また、個々の利用・配布の適法性を保証するものではありません。


# KITAQGB ライブラリ

[English](README.md) | [日本語](README.ja.md)

[日本語のライブラリ説明書](https://bartaro.github.io/kitaq-docs/gb-library.html)には、各関数の使い方とサンプルコードを掲載しています。

`wire3d_dmg` はゲームボーイ用のモノクロ・ワイヤーフレーム描画ライブラリです。128×96では `wire3d_dmg_96.c`、128×120では `wire3d_dmg.c` を選び、`Wire3DDMG_*` 関数を使います。`wire3d` と `dmg3d` は、それぞれの解像度に対応する互換用の入口として残しています。同じROMに組み込む入口は一つだけにしてください。カラー専用の `wire3d_cgb` は別のライブラリです。

[ワイヤーフレーム描画ガイド](wire3d_dmg_guide_ja.md)

このフォルダのファイルは、用途に応じて次の三つに分かれます。

- 公開API：ゲームからヘッダーを読み込み、必要なソースと一緒にビルドして使うライブラリ。
- 補助ファイル：レジスター定義、追加機能、空のソースなど。単独の公開APIとして使うものではありません。
- 参考資料：使い方を説明する文書。ROMのビルドには組み込みません。

## ファイル一覧

### 公開API

| ファイル | 用途 | 基本的な使い方 |
| --- | --- | --- |
| `physics2d.h` / `physics2d.c` | 軸に平行な矩形（AABB）による2D物理。重力を反映し、接触を反復計算で解決します。 | `physics2d.h` を読み込み、`physics2d.c` を一緒にコンパイルします。 |
| `physics2d_circle.h` / `physics2d_circle.c` | ボールを使うゲーム向けの円形物体の2D物理。 | ヘッダーを読み込み、ソースを一緒にコンパイルします。 |
| `physics3d.h` / `physics3d.c` | AABBによる3D物理。加速度、質量を考慮した反発、破壊フラグ、`kq3d_dot_q8_8()` を提供します。 | ヘッダーを読み込み、ソースを一緒にコンパイルします。 |
| `wire3d.h` / `wire3d.c` | 固定小数点による3Dワイヤーフレーム描画。WRAM上で描画し、モデルの隠線処理とシーンの遮蔽マスクを扱います。 | `wire3d.h` を読み込み、`wire3d.c` を一緒にコンパイルします。 |
| `dmg3d.h` / `dmg3d.c` | DMG向けの128×120描画。D000の作業領域にアセンブリで線を描き、STATを確認しながら8900へ転送します。 | 1bppの作業バッファを使う場合に、ヘッダーとソースを組み込みます。 |
| `wire3d_cgb.h` / `wire3d_cgb.c` | CGB専用の8MHzカラー描画。2bppのWRAMバッファ、隠線・遮蔽処理、クリッピング付きアセンブリ描画、HBlank DMAによる画面更新。 | CGB専用ROMとしてビルドし、ヘッダーとソースを組み込みます。 |
| `system.h` / `system.c` | 初期化、フレーム数、VBlank待ち、協調的なVBlankコールバック、DI/EIラッパー。 | フレーム単位のゲームループで組み込みます。 |
| `input.h` / `input.c` | ボタンの押下中・押した瞬間・離した瞬間・リピートの状態管理。 | メニュー、アクション、パズル、SLGなどの入力処理で組み込みます。 |
| `vram.h` / `vram.c` | BGタイル書き込み、矩形塗りつぶし、マップ転送、memcpy、memsetを予約するVRAMキュー。 | 処理中に更新を予約し、安全な時間帯に `vram_flush()` または `vram_flush_now()` で転送します。 |
| `sprite.h` / `sprite.c` | OAMの作業用コピー、スプライト割り当て、メタスプライト、アニメーション、OAM DMA、走査線上の個数超過の確認。 | OBJによる描画でヘッダーとソースを組み込みます。 |
| `fixed.h` / `fixed.c` | Q8.8固定小数点、`Vec2`、`KQRect`、clamp/min/max/lerp、基本的な矩形判定。 | 移動、物理、カメラ、AIの評価値などに使います。 |
| `scene.h` / `scene.c` | タイトル・ゲーム・ポーズなどのシーン表と、切り替え・更新・描画の振り分け。 | ゲーム全体の状態遷移を整理するときに組み込みます。 |
| `entity.h` / `entity.c` | 最大 `ENTITY_MAX` 個の小さなゲームオブジェクトを固定配列で管理するプール。 | コールバックにはIDが渡されます。実体は `entity_get(id)` で取得できます。 |
| `danmaku.h` / `danmaku.c` | 固定小数点の96発プール、32方向の扇状弾、命中・かすり判定、OAMの個数制限に依存しないCGB用BG合成。 | ヘッダーとソースを組み込みます。詳細は `danmaku_guide.md` とゲーム `ressen_gbc` を参照してください。 |
| `bank.h` / `bank.c` | コンパイラ組み込み命令を使う遠方データ・ポインター・関数呼び出しと、基本的なMBCバンク切り替え。 | バンクをまたぐデータアクセスに使います。 |
| `asset.h` / `asset.c` | 素材IDの記述子表と、生データ・タイルの読み込み。 | ヘッダーとソースを組み込みます。将来の `assets.h/c/json` 生成先としても使える形式です。 |
| `debug.h` / `debug.c` | KOKURAなどから確認するための、ROM内の軽量なトレース・アサート・マーカーバッファ。 | 重い集計はROM外で行い、ROM側では小さな記録だけを残します。 |
| `chain.h` / `chain.c` | ヘビ、ロープ、列車、関節スプライトなどに使う座標履歴のリングバッファ。 | 過去の位置を後続パーツへ伝える移動に使います。 |
| `cgb_tile.h` | CGBのタイル・属性を扱うコンパイラ組み込み命令の公開宣言。 | `__settile...` などを使うゲームソースから読み込みます。 |
| `cgb_palette.h` / `cgb_palette.c` | CGBのBG/OBJパレットを扱う上位API。 | ヘッダーを読み込み、ソースを一緒にコンパイルします。 |
| `scroll.h` / `scroll.c` | コンパイラ組み込み命令を使うスクロールと画面分割テーブル。 | `Scroll_*` 関数を使う場合に組み込みます。 |
| `raster.h` / `raster.c` | 帯状のラスタースクロールと、走査線ごとのX方向の変形パターン。 | `raster.h` を読み込み、`raster.c` と `scroll.c` を組み込みます。 |
| `camera.h` / `camera.c` | `scroll.*` を基にした8.8固定小数点カメラ、簡易グローバルAPI、ワールド座標と画面座標の変換。 | ヘッダーを読み込み、ソースを一緒にコンパイルします。 |
| `audio.h` / `audio.c` | BGM、効果音、パン、波形、フェードを扱う共通音源ドライバー。音符番号67（`G6`）までの68番号に対応します。 | 音を使うゲームに組み込みます。 |
| `audio_vblank.h` / `audio_vblank.c` | 同じ68音符番号を使うVBlank割り込みBGMドライバー。バンク付き楽曲向けの16レコードWRAMキューと任意のフレームフック。 | 直接ポインターの楽曲は固定バンク0へ配置します。バンク付き楽曲はキューへ補充し、`scripts/patch_gb_vblank_irq.ps1` で0040番地のベクターを設定します。 |
| `link.h` / `link.c` | 通信ケーブル用の1バイト転送と、協調的な論理4人通信 `Link4_*`。 | 通信を使うゲームに組み込みます。 |
| `link_packet.c` | `link.c` に追加するパケット層。`Link4_*` の相手別受信箱を含みます。 | パケット送受信が必要な場合だけ `link.c` と一緒にコンパイルします。 |
| `link_dmg07.h` / `link_dmg07.c` | 実物のNintendo DMG-07 Four Player Adapter向けの、外部クロックを使うポーリングドライバー。 | `link_hwregs_gb.c` と組み込みます。論理4人通信の `Link4_*` とは別です。 |
| `rpg.h` | RPG・ADV・SLGの共通宣言と、下位の組み込み命令の宣言。 | この機能群を使うゲームソースから読み込みます。 |
| `rng.c` | `rng8`、`rng16`、`rand_range`、`weighted_choice`、`rng_seed`、`rng_next8`、`rng_next16`、`rng_range`、`rng_chance`。 | `rpg.h` の乱数関数を使う場合に組み込みます。 |
| `flags.c` | 2048個のフラグを扱うビット集合と、クエスト状態の保存領域。 | `rpg.h` のフラグ・クエスト機能を使う場合に組み込みます。 |
| `rle.c` | RAMや遠方ROMから `[個数][値]` 形式のRLEデータを展開。 | `rpg.h` の `rle_decode*` 関数を使う場合に組み込みます。 |
| `text.c` | タイル文字列のウィンドウ、ページ待ち、選択肢、XY位置指定、数値表示、消去・ウィンドウ操作の別名。 | `rpg.h` の文字表示機能を使う場合に組み込みます。 |
| `menu.c` | 縦型メニュー、最小限の持ち物メニュー、処理を止めないメニュー状態API。 | `rpg.h` のメニュー機能を使う場合に組み込みます。 |
| `script.c` | RPG・ADV向けの小さなバイトコード実行器。 | `rpg.h` のスクリプト機能を使う場合に組み込みます。 |
| `map.c` | 圧縮配置されたマップの読み込み、衝突・イベント・カメラ、任意の16×16メタタイル。 | `rpg.h` のマップ機能を使う場合に組み込みます。 |
| `save.c` | ヘッダー、版、長さ、チェックサムを持つMBC5方式のSRAM保存・読み込み・確認・消去。 | `rpg.h` のセーブ機能を使う場合に組み込みます。 |
| `slg_unit.c` | SLGの移動・攻撃範囲。 | `rpg.h` のユニット機能を使う場合に組み込みます。 |
| `slg_path.c` | 幅優先探索による経路検索と、移動コストを使う到達範囲計算。 | `rpg.h` の経路機能を使う場合に組み込みます。 |
| `slg.h` / `slg_board.c` | ボードゲームや戦術ゲーム向けの盤面、手の一覧、取り消しスタック。 | `slg.h` と `slg_board.c` を組み込みます。ゲーム固有の評価処理はゲーム側へ置きます。 |

### 補助ファイル

| ファイル | 用途 | 注意点 |
| --- | --- | --- |
| `audio_hwregs_gb.c` | APUと波形RAMの最小限のレジスター宣言。 | 別のファイルですでに同じレジスターを宣言している場合は組み込みません。 |
| `link_hwregs_gb.c` | 通信用の `SB`、`SC`、`IF`、`IE` の最小限の宣言。 | 別のファイルですでに通信レジスターを宣言している場合は組み込みません。 |
| `cgb_tile.c` | 意図的に空にしてあるCGBタイル機能のソース。 | 公開宣言は `cgb_tile.h` にあります。このソースはコンパイルしても支障ありませんが、処理には不要です。 |
| `math.c` | ROM上の正弦テーブル `MATH_SIN`。 | 現時点では安定した公開APIとして扱わず、補助データとして使ってください。 |

### 参考資料

| ファイル | 内容 |
| --- | --- |
| `README.md` | この概要とビルド方法の英語版。 |
| `wire3d_guide_ja.md` | ワイヤーフレーム描画の日本語入門。 |
| `dmg3d_guide_ja.md` | DMG用の作業バッファを使う描画の日本語入門。 |
| `wire3d_cgb_guide.md` | CGB専用カラー描画の入門。 |
| `physics_guide.html` | 物理ライブラリの英語ガイド。 |
| `physics_guide_ja.html` | 物理ライブラリの日本語ガイド。 |

## ビルド方法

以下はアプリケーションのソースとライブラリを組み合わせるビルド例です。各コマンドに指定するアプリケーションのソースは利用側で用意してください。同梱の入門プログラムは `../examples/build.ps1` とHTMLマニュアルでビルドできます。短いコマンド名 `kitaqgb` は実行ファイルがPATHに登録されている場合に使えます。

ゲームのソースと、利用するライブラリのソースをまとめて指定します。

```powershell
kitaqgb hwregs.c lib/audio.c main.c lib/physics2d.c lib/physics2d_circle.c lib/physics3d.c lib/cgb_palette.c lib/scroll.c lib/camera.c -I lib -o game.gb --profile=dev
```

ワイヤーフレーム描画では、描画ライブラリをゲームのソースと一緒にコンパイルします。

```powershell
.\kitaqgb.exe lib/wire3d.c examples/wire3d_minimal.c -I lib -o examples/wire3d_minimal.gb --profile=dev --stack-bank=fixed --rst-disable --no-disasm
```

同梱の `examples/wire3d_minimal.c` はモデルの全フィールドを初期化し、128×96の画面で立方体を回転させます。外部の画像やフォントは不要です。

128×120のDMG用ゲームでは、120行用の互換入口 `dmg3d.*` も使えます。

```powershell
.\kitaqgb.exe lib/dmg3d.c examples/dmg3d_minimal.c -I lib -o examples/dmg3d_minimal.gb --profile=dev --stack-bank=fixed --rst-disable --no-disasm
```

`DMG3D_Init()` はD000のWRAM作業領域と、0x8900からのタイル転送を使って128×120の描画面を設定します。`DMG3D_BeginFrame()` がリセットするのは遮蔽状態だけです。画素は転送すると消去されます。`DMG3D_EndFrame()` はVBlankを待ってからSTATを確認しながら転送するため、VBlank終了後まで処理が続く場合があります。同梱の `examples/dmg3d_minimal.c` は変更範囲だけを転送するモードで、毎フレーム十字を描き直します。補助領域の転送は別操作で、転送元の記憶領域をメインの作業領域と共有します。

CGB専用のカラー描画には `wire3d_cgb.*` を使います。

```powershell
kitaqgb lib/wire3d_cgb.c examples/wire3d_cgb_color_demo.c -I lib -o examples/wire3d_cgb_color_demo.gbc --profile=dev --stack-bank=fixed --rst-disable --cgb=cgb_only --rom-title=CGBWIRE3D
```

`Wire3DCGB_Init()` はCGBを倍速に切り替え、128×96・2bppのBG描画面と4色の初期パレットを設定します。通常版と `Fast` 版のフレームAPIは、どちらも表示途中の書き換えによるちらつきを避ける方式です。`0xD300–0xDEFF` の3072バイトをHBlank DMAで非表示側のVRAMタイルバンクへ送り、VBlank中に表示を切り替えます。LCDCを毎フレーム変更する必要はありません。`Fast` 版では通常のフレーム終了時のBGキュー確認を省略します。色は `Wire3DCGB_SetPaletteRGB15()`、`Wire3DCGB_SetLineColor()`、`Wire3DCGB_Draw*Color()` で指定します。

CADなどで向きごとに用意した描画用データには、`Wire3DCGB_DrawMaskedModel2D()` が使えます。投影済みの符号付き頂点オフセットと、表示する辺を示すビットマスクを渡します。辺の走査とアセンブリ描画は描画ライブラリのバンク4内で行われるため、各線の描画ごとにバンクをまたいで呼び出す必要はありません。

変更の少ない画面でOAMの作業用コピーも使う場合は、VBlankに入ってから `sprite_flush_oam()`、`Wire3DCGB_EndFrameSparseNow()` の順に呼べます。最初の「次のVBlank待ち」を省けますが、変更範囲や現在の走査線によってDMAや表示切り替えで待つ場合があります。同じVBlank中にすべて終わる保証はありません。

CGBの線には色番号1・2・3を使ってください。通常の128×96描画では色のビットを重ねるので、1と2が重なった部分は3になります。色0で線を消すことはできません。フレームの消去または専用の消去関数を使います。通常の `Wire3DCGB_DrawLine2D` とモデル描画は、部分転送用の変更範囲を記録しません。範囲の記録が必要な線には `Wire3DCGB_DrawLineClipped2D` を使うか、`Wire3DCGB_InvalidateFrameHistory` で次の部分転送に画面全体を含めてください。

160×144モードで割り当てられるタイルは1フレーム最大127枚です。高速な線描画経路で割り当て失敗や範囲外座標が発生すると `Wire3DCGB_GetFullScreenOverflow()` が設定され、次のフレームリセットまで画素の書き込みを停止します。頂点は選んだ描画面の内側に置いてください。三角形マスクの余白は128×96でX=127、全画面でX=159までです。特に全画面とFastMapでは、API説明にあるWRAMバンクの割り当て条件を守ってください。

両モードの境界を確認する完成プログラムは、[CGB三角形マスクの境界テスト](../tests/library/wire3d_cgb_mask_bounds.c)にあります。

RPG・ADV・SLG機能のビルド例です。

```powershell
kitaqgb examples/example_rpg_text.c lib/text.c lib/menu.c -I lib -o text.gb --profile=dev
kitaqgb examples/example_adv_script.c lib/text.c lib/flags.c lib/script.c -I lib -o script.gb --profile=dev
kitaqgb examples/example_slg_cursor.c lib/map.c lib/slg_unit.c lib/slg_path.c -I lib -o slg.gb --profile=dev
```

標準ランタイムの基本動作確認用のビルド例です。

```powershell
kitaqgb lib/text.c lib/menu.c lib/map.c lib/scroll.c lib/camera.c lib/rng.c lib/save.c lib/system.c lib/input.c lib/vram.c lib/sprite.c lib/fixed.c lib/scene.c lib/entity.c lib/bank.c lib/asset.c lib/debug.c lib/chain.c lib/physics2d.c lib/slg_board.c examples/standard_library_smoke.c -I lib -o examples/standard_library_smoke.gb --profile=dev --rom-title=STDLIBSMK --no-disasm
```

シリアル通信では、通信コアより先にレジスター定義ファイルを指定します。

```powershell
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c main.c -I lib -o game.gb --profile=dev
```

ホストが通信相手を選ぶ協調的な論理4人通信でも、同じファイルを使います。ホストは `Link4_InitHost(slot_count)` で初期化し、`Link4_SelectPeer()` または `Link4_SendPacketTo()` で相手を選びます。子機は `Link4_InitPeer(local_slot, slot_count)` で初期化して、スロット0のホストと通信します。

スロット番号を指定済みの子機用ラッパーは、次のようにビルドできます。

```powershell
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c examples/link4_demo_peer_slot1.c -I lib -o peer1.gb --profile=dev
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c examples/link4_demo_peer_slot2.c -I lib -o peer2.gb --profile=dev
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c examples/link4_demo_peer_slot3.c -I lib -o peer3.gb --profile=dev
```

実物のDMG-07には、専用のポーリングドライバーを使います。

```powershell
kitaqgb lib/link_hwregs_gb.c lib/link_dmg07.c main.c -I lib -o dmg07.gb --profile=dev
```

`LinkDmg07_Poll()` は繰り返し呼び続けてください。アダプターのバイト転送間隔は映像の1フレームよりずっと短いためです。`LinkDmg07_TickFrame()` はVBlankごとに1回呼び、無通信・ハンドシェイク待ちの飽和カウンターを更新します。ドライバーは常に外部クロックの `SC=$80` を設定し、接続確認には `88 88 RATE 01` と応答します。`AA AA AA AA` による送信開始要求ができるのは、物理的なプレイヤー1だけです。

全機が `CC CC CC CC` を確認すると、各スロットの1バイトをまとめた4バイト単位の一斉配信が始まります。入力したデータが配信されるのは次のパケットなので、ドライバーは最初の未定義パケットを捨て、送受信のシーケンス値を公開します。`LinkDmg07_RequestRestart()` は次のパケット境界を待ち、位置をそろえた `FF FF FF FF` を送り、アダプターから4バイトすべてFFの通知を受け取った時点で停止します。転送中に無通信タイムアウトが起きた場合も、現在の4バイト内の位置を保って再開処理を予約します。クロックが戻ると、途中のパケットを完了してから復旧に進めます。

ゲーム側では必要なヘッダーを読み込みます。

```c
#include "physics2d.h"
#include "physics2d_circle.h"
#include "physics3d.h"
#include "wire3d.h"
#include "cgb_tile.h"
#include "cgb_palette.h"
#include "scroll.h"
#include "raster.h"
#include "camera.h"
#include "audio.h"
#include "audio_vblank.h"
#include "system.h"
#include "input.h"
#include "vram.h"
#include "sprite.h"
#include "fixed.h"
#include "scene.h"
#include "entity.h"
#include "bank.h"
#include "asset.h"
#include "debug.h"
#include "chain.h"
#include "slg.h"
```

## 使用上の注意

音符番号の最後の8個は、現状では一つ前のオクターブの周波数を再利用します。68個の番号が使えることと、68種類の異なる音高が鳴ることは同じではありません。

- `inv_mass_q8 == 0` は動かない物体を表します。
- `Wire3D_Init()` は128×96のBG描画面、`0xD000` からのWRAM作業領域、`0x8900` からのタイル領域を使います。`--stack-bank=fixed` を指定してビルドしてください。
- `Wire3D_BeginFrame()` はWRAMの描画バッファを消去します。`Wire3D_EndFrame()` はVBlankを待ち、STATを確認しながらVRAMへまとめて転送します。
- Wire3Dの角度は16段階です。基本のモデル描画で扱える頂点数は、1モデルにつき最大 `WIRE3D_MODEL_VERTEX_LIMIT` 個です。
- 面を持つ物体を線で描き、複数の物体が重なる場合は `Wire3D_DrawScene()` を使います。近い物体から描いて面のマスクを蓄積し、遠い線を保守的に隠します。
- `Wire3DCGB_Init()` はCGB専用で、KEY1/STOPによる倍速切り替えを行います。`--cgb=cgb_only` を指定し、DMG互換の `wire3d.*` と同じROMへ組み込まないでください。
- 通常版と `Fast` 版のフレームAPIは、非表示側のVRAMタイルバンクの転送が終わるまで前の画面を保ちます。BGタイルの更新キューを使わないシーンでは `Fast` 版が適しています。
- `NR10..NR52` と `WAVE0..WAVE15` を宣言するファイルは、`lib/audio.c` より先にコンパイルします。
- その定義には `lib/audio_hwregs_gb.c` が使えます。同じ音源レジスターを別のファイルでも定義している場合は重複して組み込まないでください。
- `cgb_tile.h` はコンパイラ組み込み命令を直接公開します。`lib/cgb_tile.c` は空のファイルなので、通常のビルドでは省略できます。
- `cgb_palette.h` の公開API名は `cgb_*` です。
- メニューや設定で音の有効・無効を変えたら、`Audio_SetMusicEnabled()` / `Audio_SetSfxEnabled()` を呼びます。
- `Audio_PlaySFX()` は呼び出し時に見えているROMバンクを記録します。効果音データのバンクが明確な場合は `Audio_PlaySFXBanked(bank, sfx, priority)` を使います。
- 楽曲ストリームの `AUDIO_CMD_NOTE` / `AUDIO_CMD_SET_INST` は次のチャンネル番号を使います。`0=CH1`、`1=CH2`、`2=CH3`、`3=CH4` です。
- CH3用の独自波形には、32個の4ビットサンプルを16バイトへ詰め、`Audio_LoadCustomWave()` に渡します。
- `Audio_FadeToMasterVolume()` のフェードは `Audio_Update()` で進みます。フェード中も毎フレーム呼んでください。
- `audio_vblank.c` はVBlank割り込みベクターのシンボル `__kq_vblank_vector` を定義します。BGMはイベントごとに `delay, ch2_note, ch1_note, ch3_note, ch4_noise_param` の5バイトで、休符・反復・終了には `AUDIO_VBLANK_REST`、`AUDIO_VBLANK_LOOP`、`AUDIO_VBLANK_END` を使います。
- 直接ポインターで渡すVBlank楽曲は固定バンクへ置いてください。キューモードなら別バンクの楽曲から補充できます。リンク後に `scripts/patch_gb_vblank_irq.ps1 <rom> <map>` を実行し、0040番地からISRへジャンプするよう設定してROMのチェックサムも更新します。
- VBlankベクター0040番地を持つ別のライブラリやゲーム用スタブと `audio_vblank.c` を併用するには、割り込みを共有する振り分け処理が必要です。
- `Scroll_SplitCommit()` はIEの `0x01 | 0x02` を自動で有効にし、コンパイラが用意するVBlank/STATハンドラーで画面分割を再生します。
- 画面分割機能は0040・0048番地を予約します。現状では独自のVBlank/STATスタブと組み合わせないでください。
- `SB`、`SC`、`IF`、`IE` の宣言ファイルは、`lib/link.c` / `lib/link_packet.c` または `lib/link_dmg07.c` より先にコンパイルします。
- その定義には `lib/link_hwregs_gb.c` が使えます。同じレジスターを別のファイルでも定義している場合は重複して組み込まないでください。
- 通信ライブラリはシリアル割り込みベクター0058番地を定義しません。割り込みモードでは、自分のスタブや振り分け処理から `Link_OnSerialIRQ()` を呼びます。
- パケット層が保持するのは1件だけです。フレーム単位のメインループからこまめに処理してください。
- `Link4_*` はホストが相手を選ぶ協調的な4人通信です。同時に通信する子機は1台なので、ホスト側で順番に切り替えます。
- `Link4_TryReadByteFrom()` / `Link4_HasPacketFrom()` は相手別の受信箱を参照します。複数の子機を順番に調べても、誰からのデータかを保てます。
- `Link_ReadPacket()` は「最新の1件」を読むAPIです。4人通信では `Link4_ReadPacketFrom()` を使います。
- `Link4_*` はNintendo DMG-07の電気的動作や通信手順を実装するものではありません。実物には `link_dmg07.c` を使い、同じROMに `link.c` と両方を組み込まないでください。
- DMG-07の `GetConnectedMask()` は物理プレイヤー1〜4をビット0〜3で表します。転送中は最後の接続確認結果を保持します。接続台数を更新できるのは接続確認の段階だけです。
- 再開要求があっても、アダプターからクロックが来ない間は処理が進みません。復旧用の通信は通常のデータとして公開せずに捨てます。単にクロックが止まったのではなく、電源の入れ直しで別の段階へ移った場合は、ドライバーと通信セッションを明示的に初期化し直してください。
- 物理ライブラリが扱うのは直線的な位置・速度です。回転運動の力学は扱いません。
- GB向けには、動く物体を少数に抑えてください。目安は8〜24個です。
- ゲームに合わせて、ワールドごとに重力・最大速度・接触解決の反復回数を調整してください。
- ビリヤードのようなゲームでは、AABB用より `physics2d_circle.*` が適しています。
- 現在の構成では、`random`、`collision`、`ui`、`tilemap`、`dialog`、`board_game`、`simple_physics` という別ライブラリを追加する必要はありません。順に `rng`、`physics2d`、`text`/`menu`、`map`、`script`、`slg`、`physics2d` を使います。
- `scene.c` と `entity.c` は、ポインターサイズの引数を関数ポインター経由で渡すことを避けています。現在のKITAQGBでは、引数なし、または1バイトのIDを渡す形式が最も安定しています。

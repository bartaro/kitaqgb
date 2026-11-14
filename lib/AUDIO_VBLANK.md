# VBlank sound driver / VBlankサウンドドライバ

## 2026-11-14 source update

`audio_vblank.c` owns the fixed-bank `__kq_vblank_vector` interrupt handler.
Compile `audio_hwregs_gb.c`, `audio.c` and `audio_vblank.c` with your game.
Install the vector using the compiler's VBlank patching path or the supplied
example build instructions. Do not install two handlers at address `0x0040`.

### Channel order and older music

Timed records are **delay, CH1 pulse, CH2 pulse, CH3 wave, CH4 noise**.
Their duration is `delay + 1` VBlanks. FF in a note column holds the current
note; FD stops that channel. FF in the delay column ends the song; FE loops.
FC introduces a control command and FB an immediate note record.

CONTROL/SET_INST IDs now match `audio.h`: **0=CH1, 1=CH2, 2=CH3, 3=CH4**.
Use `AUDIO_VBLANK_INST_CH1` through `AUDIO_VBLANK_INST_CH4`.
Invalid IDs are ignored. Note columns and NR51 pan bits have not moved.

For an older raw stream that encoded wave as 3 and noise as 2, define
`AUDIO_VBLANK_LEGACY_INST_ORDER` before including the driver implementation
and use the same configuration in its producer. Do not enable this for a new
standard stream. Existing compiled ROMs do not change.

GBSEQ `export-vblank` detects `AUDIO_VBLANK_STANDARD_INST_ORDER` and adapts
SET_INST when an older SDK is selected. It never swaps the note columns.
See [the exporter guide](../tools/gbseq/docs/KITAQGB_VBLANK.md).

### Banked music and HDMA

Direct-pointer songs must remain accessible in fixed ROM bank 0. For banked
songs, fill the 16-record (80-byte) WRAM queue from foreground code and call
the generated `GbseqVB_Pump()` at least once per frame. Monitor
`AudioVBlank_QueueUnderruns` when integrating a busy game loop.

The default queue is in switchable WRAM bank 1: the ISR temporarily selects
bank 1 and restores SVBK. With `AUDIO_VBLANK_QUEUE_WRAM0` defined for the
implementation, the queue uses fixed WRAM and the ISR now leaves SVBK alone.
This preserves a banked HBlank DMA source. Reserve the extra 80 fixed-WRAM
bytes and use a matching driver build across your project.

### 日本語

音符レコードの順番は **待ち時間・CH1矩形波・CH2矩形波・CH3波形・CH4ノイズ**です。
SET_INSTの番号も `audio.h` と同じ **0・1・2・3** に統一しました。
音符列や左右出力ビットは入れ替えません。古い生の制御レコードを再利用する場合だけ
`AUDIO_VBLANK_LEGACY_INST_ORDER` をドライバと生成側で統一して定義してください。
GBSEQの書き出しは、指定したSDKのヘッダーを調べて旧版との番号差を吸収します。

バンクをまたぐ曲は前景処理でキューへ補給します。割り込み内で曲バンクを切り替えないでください。
`AUDIO_VBLANK_QUEUE_WRAM0` を選ぶ場合、固定WRAMを80バイト使用する代わりに
割り込みでSVBKを書き換えず、進行中のHBlank DMAの転送元バンクを維持します。
通常設定は従来どおりWRAMバンク1へ切り替えてから元に戻します。

## Verification scope

See [the source synchronization checks](../SOURCE_UPDATE_20261114.md).
Automated emulator checks do not establish physical-console or listening validation.

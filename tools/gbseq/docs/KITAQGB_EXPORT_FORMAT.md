# KITAQGB向け音楽export仕様

## 1. 対象

```powershell
gbseq.exe export-kitaqgb <project.gbseq> <out_dir>
```

GBSEQ projectを4CHのGBDAW modelへ変換し、KITAQGB `lib/audio.c` が解釈する
GBDAW-Stream v1.2 extended streamを生成します。

- CLIは常にPlayer 1のtrack 0..3をexportする。
- GUIのKITAQGB exportは現在選択中のPlayerをexportする。
- 8CH/16CHを1本の通常Game Boy ROM streamへ混ぜない。
- 8CH/16CH全体はLINK16 SAV exportを使う。

## 2. 生成物

| file | 内容 |
|---|---|
| `music_bank31.c` | `song_main[]` byte stream |
| `music_decl.h` | bank defineとextern宣言 |
| `preview_stream.json` | frame、command名、argsの可読event列 |
| `export_manifest.json` | exporter version、mode、byte数、command数、warning |
| `waves_bank31.c` | custom CH3 waveがある時だけ生成 |

GBSEQ変換ではmusic bankのGUI設定を持たないため、現在はGBDAW defaultのbank 31を
使います。C出力の基本形は次の通りです。

```c
#pragma bank 31
#include "music_decl.h"

const unsigned char song_main[] = {
    0x11, 0x00, 0x02, 0xF2,
    /* ... */
    0x40,
};
```

```c
#pragma once

#define SONG_MAIN_BANK 31
extern const unsigned char song_main[];
```

## 3. stream channel ID

現行の `lib/audio.h` に合わせ、stream IDと物理channel indexは同じ順序です。

| channel | NOTE/SET_INST/STOP stream ID | SET_PAN physical index |
|---|---:|---:|
| CH1 | 0 | 0 |
| CH2 | 1 | 1 |
| CH3 | 2 | 2 |
| CH4 | 3 | 3 |

NOTE / SET_INST / STOP_CHANNEL / SET_PAN はすべて CH1=0, CH2=1, CH3=2,
CH4=3 を使います。旧出力の CH3=3 / CH4=2 のC配列は、元の `.gbseq` から
再exportしてください。`.gbseq` のtrack順序は元から CH1/CH2/CH3/CH4 のため、
project内のtrackやnoteを入れ替える必要はありません。

`audio_vblank.h` の固定長recordも `delay, ch1_note, ch2_note, ch3_note,
ch4_noise_param` の順序です。ただし本exportは `audio.c` 向けcommand streamを
生成します。VBlankの5-byte recordとは形式が異なり、そのまま置き換えては使えません。

## 4. GBDAW-Stream v1.2 extended commands

数値はすべて1 byteです。

| opcode | name | args | 意味 |
|---:|---|---|---|
| `00` | WAIT | `delay` | `delay+1` frame間隔。`0..255`、長いwaitは分割 |
| `10` | NOTE | `stream_ch, value` | CH1..3はpitch、CH4はnoise param |
| `11` | SET_INST | `stream_ch, arg0, arg1` | pulse duty+envelope、CH4 envelope等 |
| `12` | SET_PAN | `physical_ch, pan` | 0 center,1 left,2 right,3 mute |
| `13` | SET_SWEEP | `nr10` | CH1 sweep |
| `14` | SET_WAVE | `wave_id` | built-in CH3 wave 0..7 |
| `15` | SET_CH3_CUSTOM_WAVE | `16 bytes` | packed 32 x 4-bit wave |
| `16` | SET_CH4_PARAM | `nr43` | CH4 noise parameter |
| `17` | SET_MASTER_VOLUME | `left,right` | each 0..7 |
| `18` | SET_CH1_DUTY | `duty` | 0..3 |
| `19` | SET_CH2_DUTY | `duty` | 0..3 |
| `1A` | SET_CH3_LEVEL | `nr32` | `00`,`20`,`40`,`60` |
| `20` | STOP_CHANNEL | `stream_ch` | 対象CHを停止 |
| `30` | LOOP | none | stream headへ戻る |
| `40` | STOP | none | 曲を停止し全CH消音 |

unknown opcodeはfail-closedで音楽を停止し、後続byteを誤解釈しません。

### SET_INST

- CH1/CH2: `arg0=duty 0..3`, `arg1=envelope`
- CH4: `arg0`は未使用、`arg1=envelope`
- CH3 legacy path: `arg1=output level`
- extended GBSEQ exportではCH3 levelに通常 `SET_CH3_LEVEL` を使う

envelope byte:

```text
(velocity << 4) | (envelope_increase << 3) | envelope_period
```

### built-in wave ID

| ID | wave |
|---:|---|
| 0 | triangle |
| 1 | saw |
| 2 | vowel A |
| 3 | vowel I |
| 4 | vowel U |
| 5 | vowel E |
| 6 | vowel O |
| 7 | custom |

## 5. `.gbseq`からの変換

GBSEQは各noteを1つの一時instrumentへ変換し、16 row/measure、2 ticks/rowで
GBDAW engineへ渡します。

```text
tempo          = project.bpm
rows/pattern   = 16
patterns       = project.measures
order list     = 0..measures-1
frames/tick    = clamp(round(450 / bpm), 1, 15)
length_ticks   = note.length * 2
```

`frames_per_step_override` が指定されたprojectは `ticks/row=1`、
`frames/tick=frames_per_step_override`、`length_ticks=note.length` になります。
exporterはKITAQGB `Audio_ServiceMusic()` のdelay counterが次commandを
`delay+1`回目のVBlankで読むことを考慮し、absolute event間隔 `N` frameを
`WAIT, N-1` としてencodeします。このため1-frame連続eventは `WAIT, 0` です。

CH1/CH2 duty、envelope、pan、CH1 sweep、CH3 output level、CH4 noise paramは
note単位のinstrument stateからcommandへ変換されます。stateが前noteと同じ場合は
不要なcommandを省略します。

### GBSEQのCH3波形変換

`.gbseq` の `effects.wave_preset` は `wave_slots` を経由して、KITAQGB exportと
KOKURA previewの両方へ渡されます。built-in source `0..6` はproject-local
`lib/audio.c` と同じ16 byteのTriangle/Saw/A/I/U/E/O波形へ変換されます。
custom source `0x80..0xBF` は対応する `user_waves` の32 sampleをhigh nibble firstで
16 byteへpackします。

extended streamでは、波形内容が変わるstepにだけ `SET_CH3_CUSTOM_WAVE` が出力されます。
同じpresetが続く場合はwave RAMの再書き込みを省略し、noteごとの `wave_level` は
従来どおり `SET_CH3_LEVEL` へ変換されます。

## 6. previewとmanifest

`preview_stream.json`:

```json
{
  "song": "Example",
  "events": [
    {"frame":0,"cmd":"SET_INST","args":[0,2,242]},
    {"frame":0,"cmd":"NOTE","args":[0,24]},
    {"frame":6,"cmd":"WAIT","args":[6]},
    {"frame":6,"cmd":"STOP_CHANNEL","args":[0]},
    {"frame":6,"cmd":"STOP","args":[]}
  ]
}
```

`frame` は解析用absolute frameです。binaryの実時間は挿入されたWAIT列で表します。

`export_manifest.json`には少なくとも次を含みます。

```json
{
  "exporter_version": "1.2",
  "stream_mode": "extended",
  "songs": [{
    "name": "Example",
    "bank": 31,
    "bytes": 123,
    "loop_strategy": "unfoldonce",
    "commands": {},
    "warnings": []
  }],
  "waves": [],
  "sfx": []
}
```

## 7. KITAQGB ROMへの組み込み

最低限、`lib/audio_hwregs_gb.c`, `lib/audio.c`, 生成された `music_bank31.c`,
game mainを同じbuildへ含めます。

```c
#include "audio.h"
#include "music_decl.h"

void GameAudio_Init() {
    Audio_Init();
    Audio_PlayMusic(SONG_MAIN_BANK, (u8*)song_main);
}

void Game_VBlankFrame() {
    Audio_Update();
}
```

`Audio_Update()` は1 video frameに1回、通常はVBlank駆動loopから呼びます。
`audio.c` はbank 0へ置く前提です。生成されたmusic dataは宣言されたbankへ配置
されます。

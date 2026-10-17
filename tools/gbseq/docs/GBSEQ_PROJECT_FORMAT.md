# `.gbseq` JSONデータ仕様 0.2.0

## 1. 文字コードと基本形式

- UTF-8 JSON text
- 推奨拡張子: `.gbseq`
- 保存時はpretty-printed JSON
- unknown fieldはSerdeの通常動作により読み飛ばされる
- load時に `repair()`、続いて `validate()` が実行される

## 2. root object

| field | type | 制約 / 意味 |
|---|---|---|
| `version` | string | 現行 `0.2.0` |
| `title` | string | 曲名 |
| `author` | string | 作曲者/生成者 |
| `bpm` | integer | 30..300 |
| `frames_per_step_override` | integer/null | `null`: BPM駆動、`1..15`: 1 stepの正確なGame Boy frame数 |
| `measures` | integer | 1..256 |
| `link16_mode` | string enum | `local4`, `cable8`, `dmg07_16` |
| `loop_playback` | boolean | `true`: repeat, `false`: stop at the end; omitted legacy projects default to `true` |
| `tracks` | array | 4/8/16 track、完全な4CH group |
| `user_waves` | array | 0..64 entries |
| `wave_slots` | array | 必ず16 entries |
| `next_note_id` | integer | 次に割り当てる正のID |

`version` が `0.1` で始まるprojectは、旧8分音符gridとして `start` と `length`
を2倍してから `0.2.0` へ更新されます。

`repair()` は値域外の数値をclampし、不足trackやwave slotを補います。ただし生成AI
は修復を前提にせず、最初からvalidな値を出してください。

`frames_per_step_override` は、KITAQGB SFXのように毎frame値が変わる短い素材を
GBSEQ上で編集・試聴するための精密timingです。通常曲は `null` のままにします。
`1` なら1 step=1/60秒相当になり、headerには `1 FRAME/STEP` と表示されます。
このmodeはGBSEQ playback、WAV、KITAQGB export向けで、LINK16 SAV exportは
BPM同期transportとの意味が異なるため明示的に拒否されます。

## 3. Track

```json
{
  "kind": "pulse1",
  "name": "Lead",
  "mute": false,
  "solo": false,
  "notes": []
}
```

`kind` は `pulse1`, `pulse2`, `wave`, `noise` のいずれかです。各Player group内の
kind順は必ずこの順で、任意の並べ替えはできません。

modeが必要とするtrack数より多い完全groupをJSONへ残すことはvalidator上可能ですが、
active modeで使うのは先頭の1/2/4 Player分です。AI生成ではmodeとtrack数を一致
させてください。

## 4. MidiNote

```json
{
  "id": 1,
  "start": 0,
  "length": 2,
  "pitch": 24,
  "velocity": 15,
  "effects": {}
}
```

| field | type | 制約 / 意味 |
|---|---|---|
| `id` | integer | project全体でunique、通常1以上 |
| `start` | integer | 0..`measures*16-1` |
| `length` | integer | 1以上、`start+length <= total_steps` |
| `pitch` | integer | 0..47、CH4では実質使用しない |
| `velocity` | integer | 0..15、初期envelope level |
| `effects` | object | 省略時はdefault |

同一track内で `[start, start+length)` が他noteと重なってはいけません。隣接は可能
です。IDは全trackでuniqueにし、`next_note_id` は最大IDより大きくしてください。

### pitch対応

```text
0=C2, 1=C#2, ... 11=B2
12=C3, ... 23=B3
24=C4, ... 35=B4
36=C5, ... 47=B5
```

位置表示は `measure:beat.sixteenth` です。たとえばstep 0は `1:1.00`、step 15は
`1:4.75`、step 16は `2:1.00` です。

## 5. NoteEffects

全fieldをJSONに出す場合のdefaultは次の通りです。

```json
{
  "pan": "center",
  "duty": 2,
  "envelope_period": 2,
  "envelope_increase": false,
  "sweep_period": 0,
  "sweep_shift": 0,
  "sweep_negate": false,
  "wave_level": 1,
  "wave_preset": 7,
  "noise_shift": 4,
  "noise_divisor": 3,
  "noise_width7": false
}
```

| field | 値 | 主対象 | Game Boy register相当 |
|---|---|---|---|
| `pan` | `left`,`center`,`right` | 全CH | NR51 routing |
| `duty` | 0..3 | CH1/CH2 | 12.5,25,50,75% |
| `envelope_period` | 0..7 | CH1/2/4 | NRx2 bits 2..0 |
| `envelope_increase` | bool | CH1/2/4 | NRx2 bit 3 |
| `sweep_period` | 0..7 | CH1 | NR10 bits 6..4 |
| `sweep_shift` | 0..7 | CH1 | NR10 bits 2..0 |
| `sweep_negate` | bool | CH1 | NR10 bit 3 |
| `wave_level` | 1..3 | CH3 KITAQGB | 100,50,25% |
| `wave_preset` | 0..15 | CH3 LINK16 | active wave slot |
| `noise_shift` | 0..15 | CH4 | NR43 bits 7..4 |
| `noise_divisor` | 0..7 | CH4 | NR43 bits 2..0 |
| `noise_width7` | bool | CH4 | NR43 bit 3 |

CH4 byteは次式です。

```text
noise_param = (noise_shift << 4) | (noise_width7 << 3) | noise_divisor
```

対象外CHのfieldはJSONに残っていても再生へ影響しません。LINK16 SAVはCH1 sweepを
格納しません。またLINK16のenvelopeは、Playerごとに選ばれた4つのpresetへ近似
されます。

## 6. UserWaveとWavePresetSlot

### UserWave

```json
{
  "name": "Soft Triangle",
  "samples": [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,
              15,14,13,12,11,10,9,8,7,6,5,4,3,2,1,0]
}
```

- 最大64波形
- `samples` は必ず32個
- 各sampleは0..15
- SAVでは2 sampleを1 byteへhigh nibble firstでpack

### WavePresetSlot

```json
{ "source": 128, "level": 1 }
```

- `wave_slots` は必ず16個
- `source=0..6`: built-in `Triangle`, `Saw`, `A`, `I`, `U`, `E`, `O`
- `source=0x80..0xBF`: user wave index 0..63
- `level=1`: 100%
- `level=2`: 50%
- `level=3`: 25%

custom sourceは、対応する `user_waves[index]` が存在しなければなりません。

## 7. 最小のcanonical例

次は1小節LOCAL 4CHで、CH1のC4をstep 0から2 step置く例です。

```json
{
  "version": "0.2.0",
  "title": "AI Minimal",
  "author": "AI",
  "bpm": 150,
  "frames_per_step_override": null,
  "measures": 1,
  "link16_mode": "local4",
  "loop_playback": true,
  "tracks": [
    {
      "kind": "pulse1", "name": "Lead", "mute": false, "solo": false,
      "notes": [{
        "id": 1, "start": 0, "length": 2, "pitch": 24, "velocity": 15,
        "effects": {
          "pan": "center", "duty": 2,
          "envelope_period": 2, "envelope_increase": false,
          "sweep_period": 0, "sweep_shift": 0, "sweep_negate": false,
          "wave_level": 1, "wave_preset": 7,
          "noise_shift": 4, "noise_divisor": 3, "noise_width7": false
        }
      }]
    },
    {"kind":"pulse2","name":"Bass","mute":false,"solo":false,"notes":[]},
    {"kind":"wave","name":"Wave","mute":false,"solo":false,"notes":[]},
    {"kind":"noise","name":"Drums","mute":false,"solo":false,"notes":[]}
  ],
  "user_waves": [],
  "wave_slots": [
    {"source":0,"level":1},{"source":1,"level":1},
    {"source":2,"level":1},{"source":3,"level":1},
    {"source":4,"level":1},{"source":5,"level":1},
    {"source":6,"level":1},{"source":0,"level":2},
    {"source":1,"level":2},{"source":2,"level":2},
    {"source":3,"level":2},{"source":4,"level":2},
    {"source":5,"level":2},{"source":6,"level":2},
    {"source":0,"level":3},{"source":1,"level":3}
  ],
  "next_note_id": 2
}
```

## 8. 検証

```powershell
./target/release/gbseq-cli validate .\song.gbseq
```

主なvalidation errorは、track kind/order不一致、note重複、範囲外note、重複ID、
32-sampleでないuser wave、16 slotでないwave tableです。

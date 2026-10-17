# 生成AI向け Game Boy作曲ガイド

## 1. 最初に出力targetを決める

生成を始める前に、必ず次のどれかを明示します。

| target | mode | track | 納品物 |
|---|---|---:|---|
| 通常のGame Boyゲーム音楽 | `local4` | 4 | `.gbseq` + KITAQGB C stream |
| LINK16 2台 | `cable8` | 8 | `.gbseq` + MASTER/PEER SAV |
| LINK16 4台 | `dmg07_16` | 16 | `.gbseq` + P1..P4 SAV |

8CH/16CHでも、各Playerは必ずCH1,CH2,CH3,CH4を1本ずつ持ちます。同じPlayerへ
pulseを3本置く、CH3を2本置く、といった割当はできません。

## 2. 作曲上のhardware制約

### CH1

- melody、arp、accent向き
- duty 0..3で倍音を変えられる
- sweepを使える唯一のchannel
- LINK16 SAVはsweepを保存しないため、sweep必須曲は通常KITAQGB向けにする

### CH2

- bass、harmony、second lead向き
- dutyとenvelopeは使えるがsweepはない

### CH3

- bass、pad、chord tone、vocal-like lead向き
- 32 sample、各0..15
- LINK16では16 active preset、projectには64 user wave
- source変更時はwave RAM更新による短い不連続があり得る
- 同じsourceのままlevelだけ変える構成は軽い

### CH4

- kick/snare/hat/noise accent向き
- pitchではなくNR43 parameterを使う
- `noise_shift`, `noise_width7`, `noise_divisor` の組合せで音色を作る

## 3. 4CH arrangement推奨

代表例:

```text
CH1 lead / ornament
CH2 bass / counter melody
CH3 pad / long bass / chord arpeggio
CH4 drums
```

同時発音は4音までです。和音はCH1..3へ分散するか、高速arpeggioとして書きます。
CH4へ音階melodyを置かないでください。

## 4. 8CH arrangement推奨

Player間で役割を明確に分けます。

```text
P1: main lead, bass, pad, main drums
P2: harmony, sub bass, texture, percussion
```

MASTER画面からPEERの4CHも編集できますが、実際の音はP2のAPUから出ます。panは
各Game Boyのstereo output内の左右であり、複数本体の物理mix配置までは表しません。

## 5. 16CH arrangement推奨

16本を常時埋めず、Playerごとに役割と休符を設計します。

```text
P1: theme + timing anchor
P2: harmony / answer phrases
P3: bass layers / wave textures
P4: percussion / effects / accents
```

各PlayerのCH4をすべて同じstepで鳴らすとnoiseが過密になります。kick/snare/hat/
effectをPlayer間で分け、衝突を意図的に使う箇所だけ同時発音させます。

DMG-07ではeventがadapter packetへ量子化されます。極端に細かい連続音色切替より、
16分音符境界で意味のある変化を置く方が安定します。

## 6. envelope、duty、panの設計

### envelope

```text
initial level = velocity 0..15
direction     = envelope_increase
period        = envelope_period 0..7
```

LINK16ではPlayerごとに4 envelope presetへ近似されるため、1曲内で無制限に細かい
envelope値を作らないでください。各Playerで4種類以内を目安にします。

推奨例:

```text
lead:  velocity 15, decrease, period 1
bass:  velocity 12, decrease, period 2
pluck: velocity 15, decrease, period 3
soft:  velocity 8,  decrease, period 2
```

### duty

- 0 = 12.5%: thin/nasal
- 1 = 25%: bright
- 2 = 50%: square/full
- 3 = 75%: 25%の位相反転に近いcharacter

duty automationはnote/bar境界に置きます。毎step変えても再生できますが、旋律より
音色変化が目立つため、phrase単位またはaccentだけに使うのが安全です。

### pan

- `left`, `center`, `right`
- LINK16 SAV内部には `mute` codeもあるが `.gbseq` enumにはない
- bassとkickをcenter、counter phraseを左右に分けるとまとまりやすい

## 7. CH3 wave作成

32 sampleを0..15で作ります。DC offsetと急峻な段差は音色を強くします。

### triangle例

```json
[0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,
 15,14,13,12,11,10,9,8,7,6,5,4,3,2,1,0]
```

### pulse-like例

```json
[15,15,15,15,15,15,15,15,15,15,15,15,15,15,15,15,
 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]
```

### 作成ルール

- 必ず32値
- 各値0..15の整数
- LINK16で使うには `user_waves` へ追加し、`wave_slots[n].source=0x80+index`
- noteの `effects.wave_preset=n`
- levelはslot側の1/2/3

通常KITAQGB exportとKOKURA previewは、built-in 7波形とGBSEQ user waveを
`SET_CH3_CUSTOM_WAVE` へ変換します。同じ波形が続く場合はwave RAMの再書き込みを
省略します。

### frame単位SFX

毎frame `[note, envelope]` が変化するKITAQGB SFXを編集する場合は、rootへ
`"frames_per_step_override": 1` を指定します。各noteを長さ1 stepで並べると
1 note=1 Game Boy frameになります。通常曲ではこのfieldを `null` にしてください。
精密frame modeはLINK16 SAVにはexportできません。

## 8. JSON生成アルゴリズム

1. modeと小節数を決める。
2. `total_steps = measures * 16` を計算する。
3. `loop_playback`を決める。完結する曲は通常`false`、反復用BGMは`true`。
4. 必要な4/8/16 trackをPlayer-majorで作る。
5. 各trackでnoteをstart順に並べる。
6. 前noteの `start+length <= next.start` を保証する。
7. IDをproject全体で1から連番にする。
8. `next_note_id = max_id + 1` にする。
9. user waveは32値、wave slotは16 entriesにする。
10. `gbseq validate` を通す。
11. WAV previewとtarget exportを生成する。

### 重要なtarget差

- LINK16: barのlength中、同じnoteを各stepで再triggerする。
- KITAQGB: lengthをtick durationとして保持し、終了時にstopする。
- LINK16: sweepを捨てる、envelopeを4 presetへ近似する。
- KITAQGB: CLIはP1だけ、GUIは選択Playerだけをexportする。

## 9. 生成AI用プロンプト雛形

```text
次の仕様で、著作権上独立したオリジナルGame Boy曲を作成してください。

Target: LINK16 CABLE 8CH
Format: GBSEQ JSON 0.2.0
Mode: cable8
Loop playback: false
Length: 256 measures, 4096 steps
Tempo: 150 BPM
Grid: 16 sixteenth-note steps per 4/4 measure
Tracks: P1 CH1/CH2/CH3/CH4, then P2 CH1/CH2/CH3/CH4
Pitch: 0..47 = C2..B5
No overlapping notes within one track
All note IDs must be globally unique
next_note_id must be max ID + 1
CH4 must use noise_shift/noise_width7/noise_divisor
Use no more than four distinct envelope shapes per Player
Create exactly 16 wave_slots and at most 64 user_waves of exactly 32 values 0..15

Musical direction:
- [style, mood, structureをここへ記入]
- P1 owns theme and main beat
- P2 owns harmony, counterline and secondary percussion
- Leave intentional rests; do not fill all 8 channels continuously

Return one complete valid JSON object only. Do not abbreviate arrays or use comments.
```

通常ゲーム向けならTargetを `KITAQGB 4CH`, modeを `local4`, track数を4へ変更
します。16CHならmodeを `dmg07_16`, track数を16へ変更します。

## 10. 検収checklist

- JSON parse可能、commentや省略記号なし
- version `0.2.0`
- measures 1..256、BPM 30..300
- modeとtrack数が一致
- track kind順が正しい
- note IDがunique
- note重複なし
- pitch/velocity/effect値が範囲内
- user waveは32 x 4-bit
- wave slotは16個
- `gbseq validate` PASS
- `render-wav` で無音/過大音/意図しない長い休止を確認
- LINK16なら生成SAVが各131072 bytes
- KITAQGBならmanifest warningとstream byte数を確認
- 実ROMではKOKURA、最後に実機でtempo、CH3切替、pan、link同期を確認

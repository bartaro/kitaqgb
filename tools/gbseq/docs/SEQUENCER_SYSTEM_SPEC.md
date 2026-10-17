# GBSEQ / LINK16シーケンサー仕様

## 1. システム構成

`gbseq` はWindows上で曲を編集するplita版GUI/CLIです。`LINK16 SEQUENCER`
はGame Boyで動く別ROMで、GBSEQが出力したbattery SRAMを読み込みます。

```text
.gbseq JSON
   |-- export-link16-sav --> .sav x 1/2/4 --> LINK16 ROM
   |-- export-kitaqgb ----> song_main C stream --> 通常のKITAQGBゲーム
   `-- render-wav --------> Player単位の4CH WAV preview
```

GUIの`ALL16`表示ではP1..P4を同じ時刻からレンダリングし、安全のため4本を
平均ミックスして16CHを一斉試聴できます。通常のP1..P4表示では選択Playerの
4CHだけを試聴します。

1台のGame Boyに物理APUは4CHだけです。

| CH | 種類 | 主な制御 |
|---:|---|---|
| CH1 | pulse | pitch, level/envelope, duty, sweep, pan |
| CH2 | pulse | pitch, level/envelope, duty, pan |
| CH3 | wave | pitch, 32 x 4-bit waveform, output level, pan |
| CH4 | noise | NR43 noise parameter, level/envelope, pan |

LINK16は複数台を同じtransportに同期し、各本体が自分の4CHを鳴らします。
1台のAPUを8CH/16CHへ拡張するものではありません。

## 2. モード

| `.gbseq`値 | 表示 | Player | track | 最大物理CH | ROM/SAV |
|---|---|---:|---:|---:|---|
| `local4` | LOCAL 4CH | 1 | 4 | 4 | local SAV |
| `cable8` | CABLE 8CH | 2 | 8 | 8 | MASTER + PEER |
| `dmg07_16` | DMG-07 16CH | 4 | 16 | 16 | 同一ROM + P1..P4 SAV |

トラックの並びは常にPlayer-majorです。

```text
index 0..3   = P1 CH1, CH2, CH3, CH4
index 4..7   = P2 CH1, CH2, CH3, CH4
index 8..11  = P3 CH1, CH2, CH3, CH4
index 12..15 = P4 CH1, CH2, CH3, CH4
```

## 3. 時間軸

- 拍子: 4/4を前提
- 1小節: 16 step
- 1 step: 16分音符、0.25 beat
- 小節数: 1..256
- 最大: 256小節、4096 step/track
- pitch: 0..47 = C2..B5
- GBSEQ BPM: 30..300

LINK16 ROMの実効範囲は60..225 BPMです。SAVには入力BPMそのものを保存し、ROMは
Game Boyの59.7275 Hz VBlankに合わせたfractional clockを使います。

```text
denominator = bpm * 10
base_frames = floor(8959 / denominator)
remainder   = 8959 % denominator
```

各stepでremainderを累積し、必要な時だけ`base_frames+1`にします。BPM 120なら
7/8 framesを適切な比率で並べるため、整数8 frames固定による約112 BPMへの低下は
ありません。headerの`step_frames`は旧reader向けの丸め値で、現行ROMはBPM fieldを
優先します。manifestには`requested_bpm`とROM範囲へclampした`effective_bpm`を
両方記録します。

ROMのtransport addressは12-bitで0..4095です。再生開始/終了は16-step page単位で
SAVへ保存されます。`loop_playback=true`なら終了pageの後に開始pageへ戻り、`false`
なら終了pageの最後のstepを所定時間鳴らして全台を同期停止します。GBSEQ exportの
区間初期値は曲全体（P001から最終page）です。

## 4. GBSEQ編集仕様

- P1..P4タブで1台分4 trackを切り替える。
- DMG07 16CHの`ALL16`で16 trackを同一画面に表示し、PLAYで全Playerを一斉再生する。
- 再生中はplayheadが進む位置へtimelineが自動追従する。手動scroll後も再生位置が
  表示範囲を出る前に追従を再開する。
- `LOOP ON/OFF`はproject、GBSEQ preview、LINK16 SAVで共通の設定。
- note/barは同一track内で重複不可。
- `start` と `length` はstep単位。
- note/barごとに音色、pan、envelope等を保持する。
- CH3には最大64 user wave、実際にstepから選ぶactive presetは16 slot。
- `mute` / `solo` はGBSEQ previewおよびKITAQGB変換用。
  LINK16 SAVにはmute/solo状態を焼き込まず、note自体を出力する。

LINK16 SAVでは、barの `length` 区間すべてへ同じnote recordを書きます。そのため
LINK16 ROMでは各step境界で再triggerされます。通常のKITAQGB exportでは
`length_ticks` と終了eventへ変換され、長いnoteとして扱われます。

## 5. LINK16 ROM編集仕様

- 1 page = 16 step
- page = `P001`..`P256`
- Left/Right: 1 step移動
- B + Left/Right: 1 page移動
- Start + Left: 現在pageを再生開始pageに設定
- Start + Right: 現在pageを再生終了pageに設定
- Select + B: 再生中のpageへ1回でjump
- playbackによる自動page切替は行わない
- A: note/rest切替
- Up/Down: pitchまたはCH4 noise value変更
- Select + direction: CH1/CH2 dutyまたはCH3 preset変更
- B: 選択stepをaudition
- B + Up/Down: MASTERのtempo変更
- Start: MASTERから開始/停止
- Start + Select: 保存

2台MASTERは8 rowを `M1,P1,M2,P2,M3,P3,M4,P4` の順で管理します。
PEER側は原則read-onlyです。

DMG-07のPlayer 1は、local 4CH画面と `ALL16` 画面を切替できます。ALL16は
channel-major表示です。

```text
P1 CH1, P2 CH1, P3 CH1, P4 CH1,
P1 CH2, ...,
P1 CH4, P2 CH4, P3 CH4, P4 CH4
```

## 6. 音色automation

| 項目 | `.gbseq` | LINK16 SAV/ROM | KITAQGB stream |
|---|---|---|---|
| pitch | 0..47 | CH1..3 step byte | `NOTE` |
| initial level | 0..15 | envelope presetへ近似 | `SET_INST` |
| envelope period/direction | CH1/2/4 | 4 presetへ近似 | `SET_INST` |
| duty | CH1/2, 0..3 | stepごと | extended command |
| sweep | CH1 | LINK16では未格納 | `SET_SWEEP` |
| pan | 全CH | stepごと2-bit | `SET_PAN` |
| CH3 waveform | preset 0..15 | stepごと | built-in/user waveをcustom-wave commandへ変換 |
| CH3 output level | 1..3 | presetに含む | `SET_CH3_LEVEL` |
| CH4 noise | shift/width/divisor | NR43 byte | `SET_CH4_PARAM` |

LINK16のCH3 sourceが変わる時だけ、CH3を一度止めて16-byte wave RAMを更新します。
同じsourceでlevelだけ変える場合はwave RAMを再転送しません。source変更時には
Game Boy実機の仕様上、短い不連続が発生し得ます。

## 7. 同期モデル

通常ケーブルではMASTERだけがserial internal clockを出し、noteをjust-in-timeで
送らず、共通の未来stepを送ります。各本体は自分のSAVを再生します。MASTER上の
PEER editはchecked patchとして送られます。

DMG-07では全本体がexternal clockで、adapterがP1..P4の1-byte contributionを
次packetで全員へbroadcastします。P1も返送eventを受け取ってから発音するため、
P1だけ1 packet早く鳴らすことはありません。

KOKURAはbyte protocolと論理event同期を検査できますが、実DMG-07のRATE依存
inter-byte timingとsample単位のAPU位相はmodel化していません。物理4台+adapterの
最終確認は別のhardware gateです。

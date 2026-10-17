# LINK16 SRAM `.sav` format V4

## 1. 概要

| 項目 | 値 |
|---|---|
| file size | 131072 bytes / 128 KiB |
| mapper | MBC5 + battery RAM |
| magic | ASCII `L16X` |
| version | 4 |
| logical region | 32768 bytes / 32 KiB |
| region数 | 4 |
| steps/region | 4096 |
| bytes/step | 7 |
| byte order | little endian for 16-bit fields |

物理SRAM bankは8 KiBです。logical region `r` はbank `r*4` から `r*4+3`
を使用します。

```text
file 0x00000..0x07FFF = region 0
file 0x08000..0x0FFFF = region 1
file 0x10000..0x17FFF = region 2
file 0x18000..0x1FFFF = region 3
```

## 2. region header

以下のoffsetはregion先頭からの相対値です。

| offset | size | 内容 |
|---:|---:|---|
| `0x0000` | 4 | `4C 31 36 58` = `L16X` |
| `0x0004` | 1 | version = 4 |
| `0x0005` | 1 | logical slot = 0..3 |
| `0x0006` | 1 | 旧reader互換用の丸めframes per step = 4..15 |
| `0x0007` | 1 | checksum |
| `0x0008` | 2 | song steps = 1..4096, little endian |
| `0x000A` | 4 | envelope preset 0..3 |
| `0x000E` | 2 | reserved、export時 `FF FF` |

envelope preset byteはGame Boy NRx2形式です。

```text
bits 7..4 = initial volume 0..15
bit 3     = 1 increase / 0 decrease
bits 2..0 = envelope period 0..7
```

GBSEQ exporterはCH1/CH2/CH4で使用時間の長いenvelope値を集計し、上位4値を
presetにします。4種類を超える場合、各stepはbyte差が最小のpresetへ量子化されます。

## 3. step records

step `s` のoffsetは次式です。

```text
step_offset = 0x0010 + s * 7,  s = 0..4095
```

領域は `0x0010..0x700F`、end-exclusiveで `0x7010` です。

| +offset | 内容 |
|---:|---|
| `+0` | CH1 pitch 0..47、`FF`=rest |
| `+1` | CH2 pitch 0..47、`FF`=rest |
| `+2` | CH3 pitch 0..47、`FF`=rest |
| `+3` | CH4 NR43 byte、`FF`=rest |
| `+4` | packed timbre |
| `+5` | packed pan |
| `+6` | packed envelope selector |

### byte +4: timbre

```text
bits 1..0 = CH1 duty 0..3
bits 3..2 = CH2 duty 0..3
bits 7..4 = CH3 wave preset 0..15
```

### byte +5: pan

2-bit fieldをCH1から順にpackします。

```text
bits 1..0 = CH1
bits 3..2 = CH2
bits 5..4 = CH3
bits 7..6 = CH4

0=center, 1=left, 2=right, 3=mute
```

GBSEQ GUIはleft/center/rightだけを生成します。ROM内部editorは値3も循環でき、
audio driverではmuteとして扱います。

### byte +6: envelope selector

```text
bits 1..0 = CH1 preset index 0..3
bits 3..2 = CH2 preset index 0..3
bits 5..4 = CH4 preset index 0..3
bits 7..6 = reserved, 0
```

CH3にはhardware volume envelopeがないためselectorはありません。

### note lengthの変換

`.gbseq` noteの `[start, start+length)` に含まれるすべてのstepへ同じ値と音色を
書きます。LINK16 ROMは各stepでnote-onを再triggerします。長さ2のbarは、1つの
attackを2 step保持する表現ではなく、同じnoteを2回連続triggerする表現です。

## 4. CH3 wave area

`wave_base = 0x7010` です。

| offset | size | 内容 |
|---:|---:|---|
| `0x7010` | 1 | user wave count 0..64 |
| `0x7011` | 16 | preset source 0..15 |
| `0x7021` | 16 | preset output level 0..15 |
| `0x7031` | 1024 | 64 user waves x 16 packed bytes |
| `0x7431` | 1 | playback start page、0..255 |
| `0x7432` | 1 | playback end page、0..255 |
| `0x7433` | 2 | BPM、little endian、60..225 |
| `0x7435` | 1 | loop playback、0=終了pageで停止、1=開始pageへ戻る |
| `0x7436` | 3018 | reserved/padding through `0x7FFF` |

sourceは `0..6` がbuilt-in、`0x80 | user_wave_index` がcustomです。levelは
1=100%, 2=50%, 3=25%です。

custom waveは32個の4-bit sampleを16 byteへpackします。

```text
byte[i] = (sample[i*2] << 4) | sample[i*2+1]
```

## 5. checksum

すべてu8 wrapping additionです。payload index 0はregion offset `0x0010` です。

```text
sum = 0x7B
sum += logical_slot
sum += step_frames
sum += steps_lo
sum += steps_hi

for i in 0 .. 32752:
    sum += region[0x0010 + i]
    sum += i & 0xFF
    sum += (i >> 8) & 0xFF

stored_checksum = sum & 0xFF
```

注意: 現行V4 checksumはheaderのenvelope preset bytes `0x000A..0x000D` と
reserved bytesを直接はcoverしません。step/wave/paddingを含むpayload全体はcover
します。

## 6. 各ファイルのregion所有権

| mode/file | region 0 | region 1 | region 2 | region 3 |
|---|---|---|---|---|
| local | P1 local | `FF` | `FF` | `FF` |
| cable MASTER | P1 local | P2 mirror | `FF` | `FF` |
| cable PEER | P2 local | `FF` | `FF` | `FF` |
| DMG P1 | P1 local | P2 mirror | P3 mirror | P4 mirror |
| DMG P2/P3/P4 | own local | `FF` | `FF` | `FF` |

各physical Game Boyは常にregion 0を自分の演奏データとして読みます。MASTER/P1の
追加regionはremote edit用mirrorです。

## 7. 出力ファイル名

### LOCAL 4CH

```text
link16_local.sav
```

### CABLE 8CH

```text
link16_cable.p1.sav
link16_cable.p2.sav
```

2台とも同一の `link16_cable.gb` を使用します。P1側では起動時にAでMASTER、
P2側ではBでPEERを選択します。SAVは同一ROM名に対するPlayer別の兄弟fileです。

### DMG-07 16CH

```text
link16_dmg07.p1.sav
link16_dmg07.p2.sav
link16_dmg07.p3.sav
link16_dmg07.p4.sav
```

export時に `link16_sav_manifest.json` も生成されます。主要fieldは `format`,
`version`, `mode`, `measures`, `steps`, `step_frames`, `requested_bpm`,
`effective_bpm`, `playback_start_page`, `playback_end_page`, `loop_playback`,
`files` です。

GBSEQはstart=0、end=曲の最終pageで出力し、GUIの`LOOP ON/OFF`も保存します。ROMで
変更した再生区間とloop設定は同じsettings領域へ保存されます。現行ROMはBPM fieldを
優先し、59.7275 Hz VBlankに対してfractional
step clockを使うため、BPM 120を整数framesへ丸めて遅くしません。

## 8. ROMへの接続

SAVを対応ROMと同じ場所へ置き、emulator/hardware flash cartのbattery save名へ
合わせます。BGBでは同じROMを別directoryへ複製し、MASTER/PEERそれぞれに
`link16_cable.sav` を持たせます。plita KOKURA-GUIはP1 ROM stemに続く
`.p1.sav`, `.p2.sav` 等を自動探索します。

現行GBSEQは `.gbseq -> .sav` のexportを提供しますが、任意の `.sav -> .gbseq`
reverse importは提供していません。SAVを直接生成する場合も、通常はこの仕様を
手書き実装するより `gbseq export-link16-sav` を使ってください。

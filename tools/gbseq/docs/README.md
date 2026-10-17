# GBSEQ / LINK16 / KITAQGB 音楽データ仕様

このディレクトリは、plita版 `gbseq`、Game Boy用 `LINK16 SEQUENCER`、
LINK16 SRAM (`.sav`)、およびKITAQGB音楽ストリームの現行実装仕様をまとめた
資料です。人が実装を確認する用途と、生成AIへ作曲条件を渡す用途の両方を想定
しています。

## 文書一覧

1. [SEQUENCER_SYSTEM_SPEC.md](SEQUENCER_SYSTEM_SPEC.md)
   - GBSEQとLINK16 ROMの役割、チャンネル構成、長さ、操作、同期方式
2. [GBSEQ_PROJECT_FORMAT.md](GBSEQ_PROJECT_FORMAT.md)
   - JSONベースの `.gbseq` ファイルのフィールド、値域、トラック順、例
3. [LINK16_SAV_FORMAT.md](LINK16_SAV_FORMAT.md)
   - 128 KiB `.sav` のバイナリ配置、7-byte step record、波形、checksum
4. [KITAQGB_EXPORT_FORMAT.md](KITAQGB_EXPORT_FORMAT.md)
   - `export-kitaqgb` が生成するC/JSONと、GBDAW-Stream v1.2コマンド
5. [AI_MUSIC_AUTHORING_GUIDE.md](AI_MUSIC_AUTHORING_GUIDE.md)
   - 生成AIに4CH/8CH/16CH曲を書かせる際の制約、推奨構成、プロンプト雛形

## 正本と版

- `.gbseq`: format version `0.2.0`
- LINK16 SRAM: `L16X`, version `4`
- KITAQGB stream: GBDAW-Stream `v1.2`, extended mode
- 調査日: 2026-08-20

仕様の正本は次の実装です。

- `gbseq/src/model.rs`
- `gbseq/src/link16_sav.rs`
- `link16_sequencer/link16_storage.c`
- `link16_sequencer/link16_pattern.c`
- `gbdaw/crates/gbdaw-export-kitaqgb/src/lib.rs`
- `lib/audio.h`, `lib/audio.c`

古い `0.1.x` の資料やLINK16の1024-step時代の記述より、本ディレクトリの
`0.2.0` / V4 / 4096-step記述を優先してください。

## 最短ワークフロー

```powershell
cd tools/gbseq

# JSONの検証
.\target\release\gbseq.exe validate .\song.gbseq

# LINK16用の1台/2台/4台SAVを生成
.\target\release\gbseq.exe export-link16-sav .\song.gbseq .\out\link16_sav

# 通常のKITAQGBゲームへ組み込む4CH音楽Cデータを生成
.\target\release\gbseq.exe export-kitaqgb .\song.gbseq .\out\kitaqgb

# 音の事前確認
.\target\release\gbseq.exe render-wav .\song.gbseq .\out\song.wav
```

現行CLIは `export-kitaqgb --player 1..4` と `render-wav --player 1..4` に
対応します。`render-wav --all-players` は全台をミックスし、GUIのALL16表示も
16CH同時再生に対応します。LINK16 SAV exportはモード内の全Playerを一括出力します。
9言語の現行操作説明は[配布トップ](../README.md)を参照してください。
この詳細仕様は日本語の補足資料です。上記開発時パスに対し、公開コピーの
正本は `src/` と `vendor/gbdaw-export-kitaqgb/src/` にあります。


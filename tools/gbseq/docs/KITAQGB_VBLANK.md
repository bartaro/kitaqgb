# KITAQGB VBlank C export / VBlank音源用C出力

**出力 → KITAQGB VBlank (C)**、または `gbseq-cli export-vblank song.gbseq output`。通常の `export-kitaqgb`（`audio.c` のコマンド列）とは別形式です。新規フォルダに公式のドライバソース・ライセンス、各台の曲データ、キュー供給コードと単体再生例を出します。出力先のゲームや以前の書き出しは上書きしません。

**Build:** enter P1 (or P2/P3/P4) and run the command in `build.txt` with the native KITAQGB compiler on PATH. Each part builds its own `player.gb`. These examples play independently; they do not implement a link-cable transport. For integration compile the listed sources but omit `example.c`: initialize audio and VBlank, call `GbseqVB_Start()`, enable the IRQ and call `GbseqVB_Pump()` from the foreground at least once per frame. Do not install a second VBlank vector. Reserve the indicated ROM banks starting at 2. The producer copies records into WRAM before queueing, so a game does not need to keep the song bank mapped during the interrupt. Do not compile a second copy of a driver already in your game. The included KITAQGB library is supplied under its original MIT license in `driver/LICENSE`.

**Encoding:** `delay, CH1 pulse, CH2 pulse, CH3 wave, CH4 NR43`, with duration `delay + 1` VBlanks. A note byte of FF holds the channel, FD stops it. CONTROL (FC) takes a command and three arguments. Exported SET_INST IDs follow audio.h: **0=CH1, 1=CH2, 2=CH3, 3=CH4**. Duty and wave level use hardware register bits. NR51 keeps physical channel order. Timing uses cumulative VBlank rounding at the project BPM; a frame-step override is exact. Note lengths, trailing rests and loop/one-shot mode are preserved. Muting/soloing the monitor does not delete notes from source export.

**Driver selection:** use `--driver-dir PATH_TO_KITAQGB_LIB` or set `GBSEQ_KITAQGB_LIB` (GUI and CLI). Otherwise GBSEQ checks for an installed `kitaqgb-lib` directory beside its executable, then falls back to an embedded public release. On a driver with `AUDIO_VBLANK_STANDARD_INST_ORDER`, records are copied unchanged; with older headers the generated producer swaps **only** the third byte of SET_INST for wave/noise. Never swap the note columns. Legacy raw songs can use the new driver's `AUDIO_VBLANK_LEGACY_INST_ORDER` build setting instead. For a game with CH4 sound effects provide its ownership flag `Audio_NoiseEffectActive` instead of duplicating the example definition.

**番号の互換性:** 新しく出力する曲は音符もSET_INSTもCH1・CH2・CH3・CH4順です。新ドライバではそのまま、旧ドライバではキューへ渡す直前にSET_INSTの番号だけを変換します。古い生データで新ドライバを使う場合だけ互換設定を使ってください。

**Limits:** this unmodified IRQ driver can load only its triangle and saw CH3 waves. Other shapes are matched by the smallest squared difference over all 32 samples. CH4 noise parameters FD/FF conflict with stop/hold and use FC/FE respectively. Every approximation is listed in `manifest.json` and reported in the GUI. Pass `--strict` to reject such a project before any files are written. To preserve an arbitrary user wave exactly, use normal KITAQGB extended export instead.

**組み込み:** P1～P4それぞれのフォルダで `build.txt` のコマンドを実行します。ゲームに組み込む場合は `example.c` の代わりに自分のメインループを使い、初期化後に `GbseqVB_Start()`、毎フレーム前景で `GbseqVB_Pump()` を呼んでください。音符順は実機のCH1～4のままです。ドライバの波形は三角波・のこぎり波の2種で、ユーザー波形などを近似した場合は明示します。近似を認めない場合は `--strict` を使います。

## 2026-11-14 bundled driver

The embedded fallback now uses the same standard instrument IDs as `lib/` and includes the fixed-WRAM/SVBK correction. See [driver integration](../../../lib/AUDIO_VBLANK.md). Use one driver configuration throughout a game; old exported directories are not modified by updating GBSEQ.

同梱ドライバも最新 `lib/` と同じ番号体系・固定WRAM設定です。既存の書き出し先は自動更新されないため、新しいフォルダへ書き出してください。

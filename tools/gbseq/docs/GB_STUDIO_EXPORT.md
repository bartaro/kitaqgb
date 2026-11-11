# GB Studio: UGE and MOD

In **Export**, choose **GB Studio music (.uge)** or **GB Studio music (.mod)**. For a GB Studio project select a name inside its `assets/music` folder. CLI equivalents:

```sh
gbseq-cli export-uge song.gbseq song.uge
gbseq-cli export-mod song.gbseq song.mod
```

## Playback timing

If a MOD begins with four simultaneous effects, all parts receive the same one-row tempo introduction to preserve the first notes and volumes. Loops skip that introduction. If all four effect slots are occupied on a final row that needs a jump, all parts use the same extra row. Both cases are reported.

For a single console the chosen file is written. With two or four consoles the base name expands to `song_P1.uge`, `song_P2.uge`, etc. (likewise for MOD); existing numbered parts are not silently replaced. Each is an **independent** four-channel GB Studio asset, not a multi-console playback engine. Export includes all notes from all active consoles, regardless of editor view, monitor, mute or solo. The original project is unchanged.

* **UGE (recommended):** editable hUGETracker v6. CH1/CH2 duty and envelope, CH1 sweep, CH3 32-sample waveforms and levels, CH4 noise width and stereo routing are mapped to tracker instruments/effects. Up to 15 instruments of each family and 16 waveforms per file. Above the instrument limit the export reports the substitution. Noise frequencies are mapped to the nearest playable noise note.
* **MOD:** four-channel ProTracker `M.K.`, using GBT Player instrument slots and original synthetic PCM for tracker previews. GBT uses its own fixed instruments: custom waves and noise are approximated. Pan, hardware envelopes and sweep are not transferred. Prefer UGE if timbre matters. Some GB Studio versions use the GBT music-driver setting for MOD; others offer conversion to editable UGE.
* One sixteenth note is one tracker row; 64 rows form one pattern. Speeds are quantized to tracker ticks. Note lengths become note cuts; an adjacent next note retriggers instead. Looping returns to the first order; one-shot songs finish in a repeating **silent** order, not by restarting the music. The GUI lists conversion warnings; the CLI prints JSON containing output paths and warnings.

Original writers; no GB Studio, hUGETracker or GBT Player code, samples or song templates are bundled. Format references: [GB Studio music editor](https://www.gbstudio.dev/docs/assets/music/music-huge/), [GBT Player MOD format](https://github.com/AntonioND/gbt-player/blob/master/gb/rgbds/mod2gbt/mod_instructions.txt), [GB Studio UGE reader](https://github.com/chrismaltby/gb-studio/blob/eb6aa8a757203d9fdacabfc90e1e31cc5c52eb0d/src/shared/lib/uge/ugeHelper.ts).

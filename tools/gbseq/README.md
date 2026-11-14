# GBSEQ

**2026-11-14 source update:** continuous keyboard audio, Ctrl+K/piano shortcut, unsaved-song dialog, quantize fixes, refreshed player ROMs, and the latest KOKURA APU. See the [source update and checks](../../SOURCE_UPDATE_20261114.md), [English manual](README.en.md) and [日本語](README.ja.md). [Native packages updated 2026-11-15](bin/README.md) include this source update.

[MIDI files/devices, keyboard, recording and 512 instruments](docs/MIDI_PERFORMANCE.md) · `Ctrl+K` · `import-midi` / `export-midi`

ROM exports retain the 48-byte Nintendo logo data for boot-time compatibility. GBSEQ's MIT license grants no independent logo/trademark rights, and GBSEQ is not affiliated with or approved by Nintendo. [English/Japanese scope notice](docs/NINTENDO_NOTICE.md).

GB Studio: [UGE / MOD export](docs/GB_STUDIO_EXPORT.md).

[English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Español](README.es.md) | [Português](README.pt.md) | [Français](README.fr.md) | [Deutsch](README.de.md)

Game Boy sequencer by DAISUKE OBA: plita GUI, headless CLI, 4/8/16-channel editing and monitoring, WAV, KITAQGB, LINK16 saves and standalone GB/GBC player ROMs.

Choose **Export → Playback ROMs (no SRAM)** or run `gbseq-cli export-player-roms song.gbseq output_folder`. Each 512 KiB ROM contains one console’s four channels, a rendered song title and channel meters. START plays solo; SELECT while stopped opens the link role menu (A: MASTER, B: PEER). No save file is required.

- [Native downloads](bin/README.md)
- [English user manual](README.en.md) / [日本語説明書](README.ja.md)
- [MIT license](LICENSE) / [Third-party notices](THIRD_PARTY_NOTICES.md) / [License audit](LICENSE_AUDIT.json)

Choose any of the nine manual languages in **View → Language**. Double-click a track header or press E to open its key editor. WAV export follows the active mute/solo mix and the selected console monitor, not the lane being edited. Each manual covers the performance keyboard, MIDI, recording, instruments, chord display and WAV routing. No game ROM or user music is bundled.

# GBSEQ — user manual

[MIDI file and device I/O, keyboard, recording and 512 instruments](docs/MIDI_PERFORMANCE.md) · `K` · `import-midi` / `export-midi`

ROM exports retain the 48-byte Nintendo logo data for boot-time compatibility. GBSEQ's MIT license grants no independent logo/trademark rights, and GBSEQ is not affiliated with or approved by Nintendo. [English/Japanese scope notice](docs/NINTENDO_NOTICE.md).

GB Studio export: select editable UGE (including user waves) or MOD (fixed GBT instruments) in Export. Write one file per console to `assets/music`. [CLI: `export-uge` / `export-mod`](docs/GB_STUDIO_EXPORT.md).

[English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Español](README.es.md) | [Português](README.pt.md) | [Français](README.fr.md) | [Deutsch](README.de.md)

GBSEQ by DAISUKE OBA is a Game Boy sequencer with a plita desktop GUI and an independent CLI. Both share the project model, editing rules, KOKURA audio rendering and exporters. LOCAL 4CH, CABLE 8CH and DMG07 16CH support one, two and four players. Each player has CH1 pulse, CH2 pulse, CH3 wave and CH4 noise. A project holds 1–256 bars, 16 steps per bar (4096 steps/channel), BPM 30–300 and a loop on/off setting. Choose a display language in View → Language. The setting is saved independently of the song. `--language en|ja|ko|zh-CN|zh-TW|es|pt-BR|fr|de`

## Keyboard, recording and WAV

K opens the performance keyboard for the selected lane. Play white keys with A/S/D/F/G/H/J and black keys with W/E/T/Y/U; Z/X changes octave. IN and OUT cycle through MIDI devices. Step input advances by 1, 2 or 4 sixteenths; REC records one pass from the beginning. Protect skips overlapping notes; Overwrite replaces only the overlap. Import and export .mid via File and Export; keep a .gbseq for chip-specific effects.

Each channel has 128 factory instruments. Choose one from the dropdown or save and load your own .gbpreset separately. The chord overlay has separate switches for the mix and each lane; song transpose excludes noise.

WAV follows the monitor and mixer at the moment of export: All mixes the active consoles, P1–P4 isolates one. Muted tracks never sound; if any active track is soloed, only unmuted solo tracks sound. Selecting a lane for editing does not isolate it. With all tracks muted, the WAV is silent but retains its duration.

`gbseq-cli render-wav song.gbseq mix.wav` · `gbseq-cli render-wav song.gbseq p2.wav --player 2`

[VBlank C export](docs/KITAQGB_VBLANK.md): `gbseq-cli export-vblank song.gbseq output` · `--strict` · `--driver-dir`.
The records use audio.h channel order. The IRQ driver has two CH3 waveforms; use --strict to reject approximations. Keep each player’s sources together.

Press **E** or double-click a track header to edit that track on the piano roll (E returns to the overview). Drag a note’s right edge to extend it up to the next note or the song end. In the CH3 user-wave graph, draw across the 32 samples; Ctrl+Z undoes one stroke.

## Playback ROMs and listening

**Export → Playback ROMs (no SRAM)** creates one 512 KiB ROM per part. P1.gb–P4.gb each contain four voices and the song title; no save file is needed. START auditions a part alone. For sync, press SELECT while stopped on each console, choose A (MASTER) on one and B (PEER) on the others, then press START on the master. On a four-player adapter the master uses port 1, regardless of part number. In GBSEQ, **Listen: All** mixes all consoles regardless of the editing view; choose P1–P4 to isolate one. WAV export uses the current monitor.

`gbseq-cli export-player-roms song.gbseq output_folder`

## Install and start

Download your platform package from [bin/](bin/): Windows x86_64, Linux x86_64, macOS Apple Silicon or macOS Intel. Extract the entire archive, keeping assets and notices beside the programs. Run `gbseq.exe` (Windows) or `./gbseq` (macOS/Linux) for the GUI; `gbseq-cli` is the headless command-line program. In PowerShell use `./gbseq-cli.exe`; on Unix use `./gbseq-cli` unless installed in PATH. The examples below omit that prefix. A compiler, separate plita installation and Python are not required for GUI/CLI use.

The GUI needs a graphical desktop, compatible GPU driver and an audio device for playback. Linux packages target Ubuntu 24.04 or newer compatible systems and need ALSA, X11/Wayland, libxkbcommon + libxkbcommon-x11 and Vulkan/OpenGL runtime libraries. macOS binaries are unsigned and not notarized: verify their origin/hash, then use the system's per-application Open/Privacy & Security approval if required; do not disable global protection. The CLI can export audio without a sound device or display. Keep the executable permission when extracting Unix packages (`chmod +x gbseq gbseq-cli` if necessary).

## Compose in the GUI

Choose the link layout and P1–P4; ALL16 displays all sixteen lanes in DMG07 mode. Double-click an empty lane to create a note; double-click a note to open its inspector. Drag its body to move time/pitch, or its right edge to change duration. Each physical lane is monophonic: overlapping notes are rejected. Edit the bar-count box to set 1–256 bars. The mouse wheel and bottom scrollbar navigate long songs; F fits the song, Home returns to its beginning, +/- zooms. Playback follows the current position. Space starts/stops; LOOP ON/OFF determines whether the end repeats. The first playback renders the complete song, so long songs can take time to prepare.

Ctrl/Shift-click toggles a selection; Ctrl+A selects the current lane. Left/Right moves by the grid, Up/Down changes semitones, Shift+Up/Down changes octaves. Ctrl+D duplicates; Delete removes. G cycles the grid; Q quantizes starts. Ctrl+Z/Y undo/redo and Ctrl+C/V copy/paste. Ctrl+O opens, Ctrl+S saves, Ctrl+Shift+S saves as. Unsaved changes are confirmed before replacement. Keep a copy before overwriting an existing project.

## Sound and wave editing

The inspector sets each note's level and left/right/center pan, pulse duty and envelope, CH1 sweep, CH3 output level/preset, or CH4 noise parameters. AUDITION previews that note. CH3 has no hardware envelope. NEW USER WAVE opens a 32-sample 4-bit editor: drag values 0–15. Store up to 64 user waves and assign them or seven built-ins to 16 active presets with 100/50/25% level. Per-note WAVE PRESET changes automate timbre. A hardware wave change briefly disables CH3; it is not a continuous crossfade.

## Command line

CLI tracks/players are one-based; starts are zero-based steps. Length 1 is a sixteenth note, length 16 one 4/4 bar. Pitch 0–47 corresponds to C2–B5; velocity is 0–15. Use `--players 1`, `2` or `4` (3 is unsupported). Editing commands require `--out`; that explicit destination may overwrite a file. Invalid edits return a nonzero status and are not saved. `transform` also accepts `--shift`, `--quantize` and `--velocity`. WAV rendering defaults to all active consoles; `--player N` isolates one. Export operations may replace existing output files.

```sh
gbseq --open song.gbseq
gbseq-cli new song.gbseq --bars 32 --bpm 150 --players 4 --no-loop
gbseq-cli add-note song.gbseq --track 1 --start 0 --length 2 --pitch 24 --velocity 12 --out edited.gbseq
gbseq-cli notes edited.gbseq --track 1
gbseq-cli transform edited.gbseq --track 1 --transpose 12 --out higher.gbseq
gbseq-cli duplicate-notes higher.gbseq --ids 1 --out doubled.gbseq
gbseq-cli delete-notes doubled.gbseq --ids 1 --out trimmed.gbseq
gbseq-cli validate edited.gbseq
gbseq-cli info edited.gbseq --json
gbseq-cli render-wav edited.gbseq all16.wav --all-players
gbseq-cli export-kitaqgb edited.gbseq export-c --player 1
gbseq-cli export-link16-sav edited.gbseq saves
gbseq-cli bench-render edited.gbseq --all-players --frames 60 --runs 3
gbseq-cli --help
gbseq-cli transform --help
```


## LINK16 and KITAQGB export

LINK16 SAV creates 128 KiB MBC5 battery images in V4 format: `link16_local.sav`; `link16_cable.p1.sav`/`.p2.sav`; or `link16_dmg07.p1.sav` through `.p4.sav`. Use the matching LINK16 ROM/version and emulator save naming. Back up existing battery saves before replacing them. P1 includes mirrored remote parts. A SAV is data, not a bootable ROM. LINK16 allows four shared envelope presets per player; more distinct values may be approximated. Check the export and listen after loading. CH1 sweep is not carried by LINK16 V4. Exact-frame SFX timing (`frames_per_step_override`) works in preview/WAV/KITAQGB export but is rejected by the BPM-based LINK16 exporter.

KITAQGB export writes one player's C/stream data; export each player separately. Use current compatible `audio.h`/`audio.c` with physical channel IDs CH1=0, CH2=1, CH3=2, CH4=3. This does not merge sixteen channels into one Game Boy. GBSEQ preview quantizes the step duration to video frames, so arbitrary BPM values can differ from a fractional-clock ROM. Emulator rendering is not a physical-hardware test.

## Project data and Python

`.gbseq` is JSON, not MIDI. Changing an extension does not convert other formats. Validate AI-generated data and check the duration, active tracks, unique IDs, note ranges and overlaps before playback/export. Detailed Japanese specifications are in [docs/](docs/README.md). Optional Python 3.10+ bindings use the supplied native library: add `python/` to PYTHONPATH and set GBSEQ_NATIVE_LIBRARY to the absolute `gbseq.dll`, `libgbseq.dylib` or `libgbseq.so` path. Python track/player indices are zero-based; call validate after direct data edits. `player=None` renders all active players. No Python package is required for normal GUI/CLI use.

```python
from gbseq import Project
song = Project()
song.data["measures"] = 4
song.add_note(track=0, start=0, length=2, pitch=24, velocity=12)
song.validate()
song.save("python-song.gbseq")
song.render_wav("python-song.wav")
```


## Build, troubleshooting and rights

Build with Rust 1.93.1 and the platform C/C++ tools. Linux build packages include libasound2-dev, libudev-dev, libxkbcommon-dev, libxkbcommon-x11-dev, libwayland-dev, libx11-dev, libxi-dev, libxcursor-dev and Mesa development libraries. The CI workflow builds/tests each native platform and checks GUI startup, audio/export and save preservation. Verification receipts record their source commit; audio-device output and physical input/hardware are not tested by those checks.

```sh
cd tools/gbseq
cargo test --locked --release --tests
cargo build --locked --release --bin gbseq
cargo build --locked --release --no-default-features --bin gbseq-cli --lib
```


If the window fails, update the graphics driver and run from a terminal to see the error. If text is missing, restore `assets/fonts`; if there is no sound, check the output device and mute/solo. For failed exports run validate and inspect the indicated note. GBSEQ code is MIT, Copyright (c) 2026 DAISUKE OBA. Read [LICENSE](LICENSE), [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) and [LICENSE_AUDIT.json](LICENSE_AUDIT.json); keep the full notices and font licenses when redistributing. Third-party assets and imported music retain their own terms.

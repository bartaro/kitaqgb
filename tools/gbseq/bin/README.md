# Native downloads

The 2026-11-14 source changes are newer than these archived packages. Check `BUILD_VERIFICATION.json` for each package revision, or build the current source as described in the manuals.

Keyboard audition uses the current lane's selected note (including edited
duty, envelope, pan and custom CH3 wave), then its input preset or cursor
instrument. PC keys, mouse keys and MIDI share that path. The source tests
compare the keyboard's PCM with score playback for all four APU channels.

These builds include the nine-language GUI, MIDI file and device I/O, live/step recording, 512 channel instruments, chord overlays, and WAV export that matches the active mixer and console monitor. See the [performance guide](../docs/MIDI_PERFORMANCE.md) or a localized user manual. GB Studio UGE and MOD export are available in both the GUI and CLI.
See the [format guide](../docs/GB_STUDIO_EXPORT.md) and the [GB Studio importer checks](GB_STUDIO_INTEROP.json).

The separate [KITAQGB VBlank C exporter](../docs/KITAQGB_VBLANK.md) writes a
buildable package for each console. The bundled library is the published
KITAQGB snapshot; `--driver-dir` selects your installed library.

Each platform package contains both the **gbseq GUI** and **gbseq-cli**, the
optional C ABI/Python library, all nine user manuals, fonts and full licenses.
Keep the entire extracted directory together. See the manual for system
requirements and unsigned macOS application approval.

| Platform | Package |
|---|---|
| Windows x86_64 | [ZIP](gbseq-windows-x86_64.zip) |
| Linux x86_64 (Ubuntu 24.04+) | [tar.gz](gbseq-linux-x86_64.tar.gz) |
| macOS Apple Silicon | [tar.gz](gbseq-macos-arm64.tar.gz) |
| macOS Intel | [tar.gz](gbseq-macos-x86_64.tar.gz) |

[SHA-256 checksums](SHA256SUMS) · [Build and verification record](BUILD_VERIFICATION.json) · [Archive contents and CPU verification](PACKAGE_VERIFICATION.json)

Run `gbseq.exe` or `./gbseq` for the GUI; use `gbseq-cli --help` for the CLI.
The native library is optional. Binaries are built from the source commit in
each package's `verification/native.json`; that record also contains binary
hashes, audio checks, and the tested platform. `verification/gui-startup.json`
records a native GUI startup check with the Japanese editor: a white-key width
of 88 pixels and a note extended from 2 to 6 steps by synthetic pointer events.
This is not a physical mouse/keyboard or audio-device test. The executables
embed the original LINK16 export templates; no commercial games or user songs are included.

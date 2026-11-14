# Binary update — 2026-11-15

- [GBSEQ native packages](tools/gbseq/bin/README.md): Windows x86_64, Linux
  x86_64, macOS ARM64 and Intel GUI, CLI and optional C ABI/Python library.
- [HARAPEKO SHIROHEBI ROM and ZIP](apps/harapeko_shirohebi/bin/README.md):
  64 KiB GB/GBC ROM, full controls, licenses and build verification.

Both use public source `1caf80ef6346a52f15b3aaae07e97350f3e6b62b`.
The latest VBlank driver is included in the GBSEQ exporter assets and embedded
fallback. It is a source library compiled into ROMs; it has no separate PC executable.

[GBSEQ native CI](https://github.com/bartaro/kitaqgb/actions/runs/43608173869)
passed on all four platforms: source tests, native CLI/audio/export/FFI checks,
automated GUI startup and keyboard/save-prompt checks. Distribution archives
were checked for binary hashes, CPU formats, executable permissions, manuals,
licenses, fonts and the exact current VBlank library snapshot. The application
bytes are unchanged from CI; packaged manuals were refreshed for publication.

Shirohebi was rebuilt using the published compiler and current library. The
ROM header/global checksums and the title/start/pause/return-confirm/cancel/resume
flow passed KOKURA checks. The itch.io distribution is managed separately.

[Machine-readable verification](BINARY_UPDATE_20261115.json) links these checks
to source revisions and SHA-256 values. Physical consoles, physical MIDI/audio
devices and human listening were not tested.

GBSEQの4環境版と、はらぺこしろへびのROMを更新しました。VBlankドライバはGBSEQの
出力用ライブラリとROMに反映済みです。各ダウンロード先に説明書・ライセンス・
SHA-256と検証記録があります。実機と聴感による検証は行っていません。

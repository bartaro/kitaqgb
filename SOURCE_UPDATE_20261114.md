# Source update — 2026-11-14

**Follow-up, 2026-11-15:** [GBSEQ native packages](tools/gbseq/bin/README.md) and [Shirohebi ROM](apps/harapeko_shirohebi/bin/README.md) now include this update. The binary status below describes the original source-only publication.

Compared the current local sources with GitHub `main` (`3ef8bcc`).

- VBlank: standard SET_INST channel IDs, invalid-ID guard, and preservation of
  SVBK in the fixed-WRAM ISR. Both `lib/` and GBSEQ's embedded driver are updated.
- GBSEQ: continuous keyboard monitoring, Ctrl+K/piano control, unsaved-song
  confirmation, quantize and instrument refinements, player loop/time display,
  and the current KOKURA audio implementation. Required vendored crate source is
  refreshed while retaining portable manifests, font discovery and licenses.
- Shirohebi: existing public game controls already match the local implementation.
  Added the current music/driver compatibility guard; retained public source
  comments and the standalone build script.

## Checks on the public source

- GBSEQ `cargo test --offline --locked --tests`: 106 passed,
  0 failed, 7 ignored by default.
- Separately ran the ignored `kokura_audio` PCM comparison with the CLI freshly
  built from the updated public KOKURA source: exact sample-for-sample match.
- Built and emulated VBlank exports with standard, legacy-instrument and fixed
  WRAM configurations. All four channel registers/notes, final stop and zero
  queue underruns passed. The exporter used for this SDK check was the current
  local CLI; its exporter implementation matches this public source.
- Built Shirohebi with the public compiler and updated public SDK (64 KiB ROM),
  then checked title, START, pause, return confirmation, B cancel and resume
  through the freshly built public KOKURA C API. Input tests wait for the game's
  release lock and screen transition; they do not inject state or memory writes.

Reproduce the VBlank checks with `tools/gbseq/scripts/verify_vblank_export.py`;
provide `--cli`, `--compiler`, `--library`, `--kokura` and `--kokura-library`.
Add `--legacy` or `--fixed-wram` for the alternate builds.
Build the game with `apps/harapeko_shirohebi/build.ps1`.

## 日本語

ライブラリ・GBSEQ・しろへびをGitHub最新版と照合し、ソースと説明書を更新しました。
公開コピーでGBSEQテスト、KOKURAとのPCM一致、VBlankの3設定としろへびの基本操作を確認しました。
実機、物理的なキー入力、聴感による検証は行っていません。

The archived desktop binaries in `bin/` retain their existing recorded source
revisions. This is a source/documentation update; rebuild to use the new desktop
code. The two original embedded playback-ROM templates are updated because the
current exporter requires their loop/time-display capability bits.

The freshly built public GBSEQ CLI also passed the standard check using its
embedded fallback, producing the same ROM bytes as the updated installed SDK.

[VBlank integration](lib/AUDIO_VBLANK.md) · [GBSEQ Japanese manual](tools/gbseq/README.ja.md) · [Shirohebi](apps/harapeko_shirohebi/README.md)


[Source hashes and test counts](SOURCE_UPDATE_20261114.json).

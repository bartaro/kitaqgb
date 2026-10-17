# GBHUA Desktop Packages

[Downloads and installation](../../GUI.md) |
[Japanese manual](../../docs/manual.ja.md) |
[All nine manuals](../../GUI.md#manuals)

| Platform | Archive | SHA-256 and file manifest |
| --- | --- | --- |
| Windows x64 | [ZIP](windows-x86_64/gbhua-gui-windows-x86_64.zip) | [JSON](windows-x86_64/gbhua-gui-windows-x86_64.json) |
| Mac Apple Silicon | [tar.gz](macos-arm64/gbhua-gui-macos-arm64.tar.gz) | [JSON](macos-arm64/gbhua-gui-macos-arm64.json) |
| Mac Intel | [tar.gz](macos-x86_64/gbhua-gui-macos-x86_64.tar.gz) | [JSON](macos-x86_64/gbhua-gui-macos-x86_64.json) |

These archives are built from source commit
[`ef2be9c8f91eae7d4d7807f8a1955d85a14fc0d6`](https://github.com/bartaro/kitaqgb/commit/ef2be9c8f91eae7d4d7807f8a1955d85a14fc0d6)
by the [native GUI workflow](https://github.com/bartaro/kitaqgb/actions/runs/39060214429).
The workflow runs 54 Rust tests, four packaging/manual tests and a Python-binding
save/image-import round trip on each target. This does not establish interactive
GUI operation on every computer, OS version or GPU.

Extract the entire archive. The application includes its fonts and needs no
Python or Rust installation. Windows opens without a console and requires the
Microsoft Visual C++ x64 runtime described in the installation guide.
The Mac applications are ad-hoc signed, but not Developer ID signed or notarized.

Each archive includes nine manuals, the application's MIT license, and the
separate dependency/font/icon license notices. Author: DAISUKE OBA.

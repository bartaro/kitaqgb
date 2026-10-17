# Native downloads

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

[SHA-256 checksums](SHA256SUMS) · [Build and verification record](BUILD_VERIFICATION.json)

Run `gbseq.exe` or `./gbseq` for the GUI; use `gbseq-cli --help` for the CLI.
The native library is optional. Binaries are built from the source commit in
each package's `verification/native.json`; that record also contains binary
hashes, audio checks, and the tested platform. `verification/gui-startup.json`
records a native GUI startup check, not physical mouse/keyboard or audio-device
testing. No commercial music, game ROM or LINK16 ROM is included.

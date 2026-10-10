# GBHUA CLI Native Binaries

| Platform | Executable bundle | Build and checksum receipt |
| --- | --- | --- |
| macOS Apple Silicon | [Download](https://github.com/bartaro/kitaqgb/raw/refs/heads/main/tools/gbhua/bin/macos-arm64/gbhua-macos-arm64.tar.gz) | [ARM64 JSON](macos-arm64/gbhua-macos-arm64.json) |
| macOS Intel | [Download](https://github.com/bartaro/kitaqgb/raw/refs/heads/main/tools/gbhua/bin/macos-x86_64/gbhua-macos-x86_64.tar.gz) | [x86_64 JSON](macos-x86_64/gbhua-macos-x86_64.json) |

[Installation / 起動手順](../MACOS.md) | [Multilingual manuals](../README.md)

These are the CLI, not the GUI. Each archive contains an executable named `gbhua`,
nine translated manuals, MIT license, dependency notices and Rust standard-library
notices. No Rust or Python installation is required to use the executable.

Source commit: `505aa135e8205988fb4265b17b89d263aad46c56`.
These builds fix automatic CGB palette selection that discarded rare highlights.
Both architectures passed nine Rust tests and 21 commands using the executable
extracted from the final archive on macOS 15.7.9. Deployment target: macOS 11.0;
macOS 11 itself was not runtime-tested. No Apple Developer ID signing or notarization.

[Native test run](https://github.com/bartaro/kitaqgb/actions/runs/38070957116)

Downloads are ordinary repository files, not expiring Actions artifacts or GitHub
Releases. Verify the archive SHA-256 against its JSON receipt before running.

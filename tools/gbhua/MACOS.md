# GBHUA CLI for macOS

## Download / ダウンロード

These are terminal executables, not the GUI application. Rust and Python are not
needed to run them. Each archive includes the executable, nine translated manuals,
MIT license, dependency notices and Rust standard-library notices.

これはターミナルで使うCLI版です。GUIアプリではありません。実行時にRustやPythonの
インストールは不要です。実行ファイル、9言語の説明書、MITライセンス、依存ライブラリと
Rust標準ライブラリの権利表記を同梱しています。

| Mac | Archive / 配布ファイル |
| --- | --- |
| Apple Silicon (M-series) | [gbhua-macos-arm64.tar.gz](https://github.com/bartaro/kitaqgb/raw/refs/heads/main/tools/gbhua/bin/macos-arm64/gbhua-macos-arm64.tar.gz) |
| Intel | [gbhua-macos-x86_64.tar.gz](https://github.com/bartaro/kitaqgb/raw/refs/heads/main/tools/gbhua/bin/macos-x86_64/gbhua-macos-x86_64.tar.gz) |

The deployment target is macOS 11.0. Native tests run on macOS 15 for both CPUs;
macOS 11 itself has not been runtime-tested. Apple Silicon uses native ARM64 code,
without Rosetta. Check your chip in Apple menu > About This Mac.

ビルド時の最低OS設定はmacOS 11.0です。両CPUともmacOS 15上で実行テストしますが、
macOS 11そのものでの動作検証はしていません。Apple Silicon版はRosetta不要です。
CPUはAppleメニューの「このMacについて」で確認できます。

## Run / 起動

Apple Silicon, from the directory containing the downloaded archive:

```sh
tar -xzf gbhua-macos-arm64.tar.gz
cd gbhua-macos-arm64
./gbhua --help
./gbhua import-image /path/to/artwork.png --out scene.gbh --mode cgb
./gbhua validate scene.gbh
./gbhua preview scene.gbh --out preview.png --scale 2
./gbhua export scene.gbh --out scene.c --prefix scene
```

For Intel, replace `macos-arm64` with `macos-x86_64` in the first two lines.
Use `./gbhua` in place of `./target/release/gbhua` in the manuals. The archive
preserves executable permission; if a copying tool removes it, run `chmod +x gbhua`.
To remove the CLI, delete its extracted directory; it does not install a service.

ターミナルでダウンロード先に移動し、上記を実行します。Intelの場合は最初の2行の
`macos-arm64`を`macos-x86_64`に置き換えます。各説明書の`./target/release/gbhua`は
配布版では`./gbhua`と読み替えてください。通常は実行権限付きで展開されます。
コピー等で実行権限が失われた場合だけ`chmod +x gbhua`を実行します。
アンインストールは展開先フォルダの削除だけです。

## Verification / 検証

Next to each archive in [bin](https://github.com/bartaro/kitaqgb/tree/main/tools/gbhua/bin)
is a JSON receipt with `archive_sha256`, `binary_sha256`, `source_commit`, tested OS,
CPU architecture and executed CLI commands. Compare the downloaded file with it:

```sh
shasum -a 256 gbhua-macos-arm64.tar.gz
```

配布ファイルと同じフォルダのJSONにSHA-256、ソースのコミット、検証OS、CPU、
実行したコマンドを記録しています。上記でダウンロードしたファイルのハッシュを照合できます。

No Apple Developer ID signature or notarization is included. Gatekeeper can warn
or block the first launch. Verify the origin and checksum before deciding whether
to allow this specific executable, following [Apple's instructions](https://support.apple.com/en-us/102445).
Do not disable Gatekeeper globally.

Apple Developer IDによる署名・公証は行っていません。初回起動で警告またはブロックが
出る場合があります。配布元とハッシュを確認し、信頼できる場合だけ
[Appleの案内](https://support.apple.com/ja-jp/102445)に沿って、この実行ファイルに限り
起動を許可してください。Gatekeeper全体を無効にする必要はありません。

## Rebuild

The repository workflow builds both targets on native Mac runners, runs
`cargo test --release --locked`, and tests the executable extracted from the final
archive with PNG import, inspect, validate, preview, C/2bpp/GBTB/GBMB exports in
DMG and CGB modes. The packaging helper needs Python 3 and the Rust toolchain;
end users do not.

```sh
MACOSX_DEPLOYMENT_TARGET=11.0 cargo build --release --locked
```

Runner architecture follows [GitHub's runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).
The deployment target follows [Rust's macOS target documentation](https://doc.rust-lang.org/rustc/platform-support/apple-darwin.html).

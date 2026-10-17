# License supplements

Some registry archives omit their upstream license files. These version-specific
directories retain the corresponding upstream notices, referenced by exact
commit in `SOURCES.json`. Every file has a recorded SHA-256.

For crates which declare MIT but provide only a license link, the unmodified
SPDX standard MIT text is included separately as `MIT-standard.txt`. Its
placeholder copyright line is a template, not a claim about the package's
copyright owner. Attribution and the original license declaration remain in
`Cargo.toml.upstream`, the retained upstream notices, and `GUI-DEPENDENCIES.json`.
No upstream notice or copyright line has been replaced.

The objc2 upstream SDK licensing discussion is preserved in `LICENSE.md`.
Apple SDK files are not bundled. Builds use the native runner's Xcode SDK.
This collection documents the dependency terms; it is not legal advice and
does not relicense dependencies under GBHUA's MIT license.

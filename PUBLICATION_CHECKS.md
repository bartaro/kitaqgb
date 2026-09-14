# Publication checks

Workspace test record: September 12, 2026, on Windows x86_64.
The executable build record is identified separately in `BINARY_BUILD.json`.

Release build passed with no warnings. All GB manual examples and standalone hello/entity builds passed.

Private inputs, generated ROMs, build caches and debug symbols are excluded from Git. The root Release executable and its runtime configuration are included for Windows with .NET Framework 4.8. These checks do not cover physical hardware.

## Compiler source layout and executable

Compiler C# files, app.config and the project file are in `kitaqgb/`.
Building the project copies the Release executable to the repository root.
See `BINARY_BUILD.json` for binary hashes, build results and sample checks.
Emulator test conditions are documented in the
[manual verification records](https://bartaro.github.io/kitaq-docs/PUBLICATION_CHECKS.md).

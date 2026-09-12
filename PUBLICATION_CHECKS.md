# Publication checks

Source snapshot: 2026-09-12. Windows x86_64 checks against this publication tree.

Release build passed with no warnings. All GB manual examples and standalone hello/entity builds passed.

GUI frontends and PLITA are deferred. Private inputs, generated ROMs, build caches and debug symbols are excluded from Git. The root Release executable and its runtime configuration are included as of September 13, 2026. This packaging pass does not claim physical-hardware validation.

## Compiler source layout and executable distribution — 2026-09-13

Compiler C# files, app.config and the project file now live in `kitaqgb/`.
The source contents are unchanged except for the project output-copy target.
See `BINARY_BUILD.json` for a fresh Release build and sample-compilation results.
These checks concern packaging and compilation; emulator and hardware behavior
were not rerun for this directory change.

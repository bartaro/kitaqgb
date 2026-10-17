# Third-Party Notices

The published GUI includes only the five core crates in `vendor/plita`.
Their original MIT notice is retained at `vendor/plita/LICENSE`; font licenses
are under `vendor/plita/licenses/fonts`. Lucide/Feather notices are retained
in `licenses/lucide/LICENSE` and with their vendored SVG sources.
GUI binary archives include target-specific `GUI-DEPENDENCIES.json` and the
original registry license/NOTICE files plus Rust standard-library notices.
The application MIT license does not replace these third-party terms.

Versioned upstream supplements and their source hashes are in
`licenses/overrides/SOURCES.json`. For packages that publish only a license
declaration/link, the standard license text is provided alongside the unchanged
upstream notice and package metadata. The objc2 upstream Apple SDK discussion
is retained; no Apple SDK is redistributed. See `licenses/overrides/README.md`.

GBHUA original application code is MIT, Copyright (c) 2026 DAISUKE OBA.
This does not relicense dependencies, fonts, imported images or game assets.

The GUI uses plita, MIT, Copyright (c) 2026 PLITA UI Contributors.
Keep the upstream `plita/LICENSE` notice. Its bundled Roboto and Roboto Mono
fonts retain SIL Open Font License 1.1 terms and upstream notices under
`plita/licenses/fonts`. The GUI uses the default Roboto font.

The headless CLI does not link plita or bundle these fonts. Its registry
dependencies must be distributed with their original license/NOTICE files.
The generated `DEPENDENCIES.json` and `licenses/dependencies` in the CLI
package document the audited dependency closure, including build dependencies.
Rust standard-library notices accompany binary distributions separately.

GBTD/GBMB interoperability is implemented by reading/writing object records;
the GBHUA package does not include the original editors' source or binaries.
Supported exchange records are documented in the manuals. Import/export does
not imply that unrelated game artwork is licensed for redistribution.

References: [MIT](https://opensource.org/license/mit),
[Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0),
[SIL OFL](https://openfontlicense.org/open-font-license-official-text/).

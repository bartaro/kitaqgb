# Third-Party Notices

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

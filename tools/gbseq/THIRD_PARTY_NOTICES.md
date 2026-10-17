# Third-party notices — GBSEQ

GBSEQ application: Copyright (c) 2026 DAISUKE OBA, under the [MIT License](LICENSE).
Third-party software, fonts and icons retain their original authors and terms.
GBSEQ's MIT license does not relicense third-party material or imported music.

## Bundled components and assets

The required GBDAW model/export/audio libraries and KOKURA core are included
as MIT source in `vendor/`. No KOKURA GUI, game ROM, boot ROM, commercial
music, sound sample or user project is included. Hardware names identify
compatibility and do not imply endorsement.

The five plita foundation crates retain their upstream MIT notice. This
snapshot makes Cargo manifests self-contained, enables Metal on macOS,
and adds executable-relative font discovery. Public API and environment
variable names (for example `PlitaProgram`, `PLITA_UI_FONT`) remain unchanged.

Unmodified Roboto and Roboto Mono fonts are distributed under SIL OFL 1.1,
with their original copyrights in `assets/fonts/*-OFL.txt` and `licenses/fonts/`.
Fonts must retain those terms when redistributed; they are not under MIT.
See the [official OFL text](https://openfontlicense.org/open-font-license-official-text/).
Lucide icons and Feather-derived paths retain ISC and MIT notices in
`licenses/assets/Lucide-Feather.txt` and `vendor/plita_ui/assets/lucide/LICENSE`.
See [Lucide's upstream license](https://github.com/lucide-icons/lucide/blob/main/LICENSE).
Operating-system fonts are only loaded at runtime and are not redistributed.

## Dependency choices and binary redistribution

`LICENSE_AUDIT.json` records the GUI/CLI dependency graphs for Windows x86_64,
Linux x86_64, macOS ARM64 and macOS x86_64. Registry versions are pinned by
`Cargo.lock`. Full upstream notices are retained under `licenses/dependencies/`;
Rust standard-library notices are under `licenses/rust-standard-library/`.
For OR expressions, the MIT option is chosen where available (otherwise the
permissive Apache/BSD/Zlib option). AND expressions are cumulative: in particular,
`dpi` retains both Apache and MIT notices, and `unicode-ident` retains Unicode
terms too. The optional LGPL alternative of `r-efi` is not selected. Audio
decoder features are disabled: GBSEQ synthesizes PCM and does not require them.

Keep `LICENSE`, this file, `LICENSE_AUDIT.json`, `licenses/` and font notices
with binary redistributions. OS libraries/frameworks (Windows, Apple, Linux
ALSA/X11/Wayland and graphics drivers) are runtime dependencies, not bundled.
Each native package additionally retains its build toolchain's Rust notices.
This review covers the files and resolved versions in this distribution;
future dependency changes require another audit.

Some older macOS binding crates omit a standalone license in the registry
archive. `licenses/UPSTREAM_NOTICE_PROVENANCE.json` records the official source
revisions, fetched files and hashes used to supplement them. For objc2 versions
that declare MIT but provide only a licensing discussion, the exact discussion
is retained alongside the same project's later full MIT text, clearly labeled
as supplemental. The retained upstream discussion mentions Apple SDK-derived
declarations; Apple SDKs/frameworks are not redistributed in this package.
`dispatch` has an explicit MIT declaration and named author but no upstream
license file; its supplemental NOTICE records that distinction. This inventory
does not claim legal clearance of arbitrary future inputs or dependencies.

## Inventory

| Component | Version | Declared license | Full notice |
|---|---|---|---|
| ab_glyph | 0.2.32 | Apache-2.0 | [notice](licenses/dependencies/ab_glyph-0.2.32/LICENSE) |
| ab_glyph_rasterizer | 0.1.10 | Apache-2.0 | [notice](licenses/dependencies/ab_glyph_rasterizer-0.1.10/LICENSE) |
| adler2 | 2.0.1 | 0BSD OR MIT OR Apache-2.0 | [notice](licenses/dependencies/adler2-2.0.1/LICENSE-0BSD) |
| ahash | 0.8.12 | MIT OR Apache-2.0 | [notice](licenses/dependencies/ahash-0.8.12/LICENSE-APACHE) |
| aho-corasick | 1.1.4 | Unlicense OR MIT | [notice](licenses/dependencies/aho-corasick-1.1.4/COPYING) |
| allocator-api2 | 0.2.21 | MIT OR Apache-2.0 | [notice](licenses/dependencies/allocator-api2-0.2.21/LICENSE-APACHE) |
| alsa | 0.9.1 | Apache-2.0/MIT | [notice](licenses/dependencies/alsa-0.9.1/LICENSE-APACHE) |
| alsa-sys | 0.3.1 | MIT | [notice](licenses/dependencies/alsa-sys-0.3.1/LICENSE) |
| anstream | 1.0.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/anstream-1.0.0/LICENSE-APACHE) |
| anstyle | 1.0.14 | MIT OR Apache-2.0 | [notice](licenses/dependencies/anstyle-1.0.14/LICENSE-APACHE) |
| anstyle-parse | 1.0.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/anstyle-parse-1.0.0/LICENSE-APACHE) |
| anstyle-query | 1.1.5 | MIT OR Apache-2.0 | [notice](licenses/dependencies/anstyle-query-1.1.5/LICENSE-APACHE) |
| anstyle-wincon | 3.0.11 | MIT OR Apache-2.0 | [notice](licenses/dependencies/anstyle-wincon-3.0.11/LICENSE-APACHE) |
| anyhow | 1.0.104 | MIT OR Apache-2.0 | [notice](licenses/dependencies/anyhow-1.0.104/LICENSE-APACHE) |
| arrayref | 0.3.9 | BSD-2-Clause | [notice](licenses/dependencies/arrayref-0.3.9/LICENSE) |
| arrayvec | 0.7.8 | MIT OR Apache-2.0 | [notice](licenses/dependencies/arrayvec-0.7.8/LICENSE-APACHE) |
| as-raw-xcb-connection | 1.0.1 | MIT OR Apache-2.0 | [notice](licenses/dependencies/as-raw-xcb-connection-1.0.1/LICENSE-APACHE) |
| ash | 0.38.0+1.3.281 | MIT OR Apache-2.0 | [notice](licenses/dependencies/ash-0.38.0+1.3.281/LICENSE-APACHE) |
| autocfg | 1.5.1 | Apache-2.0 OR MIT | [notice](licenses/dependencies/autocfg-1.5.1/LICENSE-APACHE) |
| bincode | 1.3.3 | MIT | [notice](licenses/dependencies/bincode-1.3.3/LICENSE.md) |
| bindgen | 0.72.1 | BSD-3-Clause | [notice](licenses/dependencies/bindgen-0.72.1/LICENSE) |
| bit-set | 0.8.0 | Apache-2.0 OR MIT | [notice](licenses/dependencies/bit-set-0.8.0/LICENSE-APACHE) |
| bit-vec | 0.8.0 | Apache-2.0 OR MIT | [notice](licenses/dependencies/bit-vec-0.8.0/LICENSE-APACHE) |
| bitflags | 1.3.2 | MIT/Apache-2.0 | [notice](licenses/dependencies/bitflags-1.3.2/LICENSE-APACHE) |
| bitflags | 2.13.1 | MIT OR Apache-2.0 | [notice](licenses/dependencies/bitflags-2.13.1/LICENSE-APACHE) |
| block | 0.1.6 | MIT | [notice](licenses/dependencies/block-0.1.6/LICENSE-MIT) |
| block2 | 0.5.1 | MIT | [notice](licenses/dependencies/block2-0.5.1/LICENSE.txt) |
| block2 | 0.6.2 | MIT | [notice](licenses/dependencies/block2-0.6.2/LICENSE.md) |
| bytemuck | 1.25.2 | Zlib OR Apache-2.0 OR MIT | [notice](licenses/dependencies/bytemuck-1.25.2/LICENSE-APACHE) |
| bytemuck_derive | 1.11.0 | Zlib OR Apache-2.0 OR MIT | [notice](licenses/dependencies/bytemuck_derive-1.11.0/LICENSE-APACHE) |
| calloop | 0.13.0 | MIT | [notice](licenses/dependencies/calloop-0.13.0/LICENSE.txt) |
| calloop-wayland-source | 0.3.0 | MIT | [notice](licenses/dependencies/calloop-wayland-source-0.3.0/LICENSE.txt) |
| cc | 1.2.62 | MIT OR Apache-2.0 | [notice](licenses/dependencies/cc-1.2.62/LICENSE-APACHE) |
| cexpr | 0.6.0 | Apache-2.0/MIT | [notice](licenses/dependencies/cexpr-0.6.0/LICENSE-APACHE) |
| cfg-if | 1.0.4 | MIT OR Apache-2.0 | [notice](licenses/dependencies/cfg-if-1.0.4/LICENSE-APACHE) |
| cfg_aliases | 0.2.2 | MIT | [notice](licenses/dependencies/cfg_aliases-0.2.2/LICENSE) |
| clang-sys | 1.8.1 | Apache-2.0 | [notice](licenses/dependencies/clang-sys-1.8.1/LICENSE.txt) |
| clap | 4.6.4 | MIT OR Apache-2.0 | [notice](licenses/dependencies/clap-4.6.4/LICENSE-APACHE) |
| clap_builder | 4.6.2 | MIT OR Apache-2.0 | [notice](licenses/dependencies/clap_builder-4.6.2/LICENSE-APACHE) |
| clap_derive | 4.6.4 | MIT OR Apache-2.0 | [notice](licenses/dependencies/clap_derive-4.6.4/LICENSE-APACHE) |
| clap_lex | 1.1.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/clap_lex-1.1.0/LICENSE-APACHE) |
| codespan-reporting | 0.12.0 | Apache-2.0 | [notice](licenses/dependencies/codespan-reporting-0.12.0/LICENSE) |
| color | 0.3.3 | Apache-2.0 OR MIT | [notice](licenses/dependencies/color-0.3.3/LICENSE-APACHE) |
| colorchoice | 1.0.5 | MIT OR Apache-2.0 | [notice](licenses/dependencies/colorchoice-1.0.5/LICENSE-APACHE) |
| core-foundation | 0.10.1 | MIT OR Apache-2.0 | [notice](licenses/dependencies/core-foundation-0.10.1/LICENSE-APACHE) |
| core-foundation | 0.9.4 | MIT OR Apache-2.0 | [notice](licenses/dependencies/core-foundation-0.9.4/LICENSE-APACHE) |
| core-foundation-sys | 0.8.7 | MIT OR Apache-2.0 | [notice](licenses/dependencies/core-foundation-sys-0.8.7/LICENSE-APACHE) |
| core-graphics | 0.23.2 | MIT OR Apache-2.0 | [notice](licenses/dependencies/core-graphics-0.23.2/COPYRIGHT) |
| core-graphics-types | 0.1.3 | MIT OR Apache-2.0 | [notice](licenses/dependencies/core-graphics-types-0.1.3/LICENSE-APACHE) |
| core-graphics-types | 0.2.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/core-graphics-types-0.2.0/LICENSE-APACHE) |
| coreaudio-rs | 0.11.3 | MIT/Apache-2.0 | [notice](licenses/dependencies/coreaudio-rs-0.11.3/LICENSE-APACHE) |
| coreaudio-sys | 0.2.17 | MIT | [notice](licenses/dependencies/coreaudio-sys-0.2.17/LICENSE) |
| cpal | 0.15.3 | Apache-2.0 | [notice](licenses/dependencies/cpal-0.15.3/LICENSE) |
| crc32fast | 1.5.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/crc32fast-1.5.0/LICENSE-APACHE) |
| cursor-icon | 1.2.0 | MIT OR Apache-2.0 OR Zlib | [notice](licenses/dependencies/cursor-icon-1.2.0/LICENSE-APACHE) |
| dasp_sample | 0.11.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/dasp_sample-0.11.0/dasp-LICENSE-MIT) |
| dispatch | 0.2.0 | MIT | [notice](licenses/dependencies/dispatch-0.2.0/NOTICE.txt) |
| dispatch2 | 0.3.1 | Zlib OR Apache-2.0 OR MIT | [notice](licenses/dependencies/dispatch2-0.3.1/LICENSE-MIT-upstream.txt) |
| dlib | 0.5.3 | MIT | [notice](licenses/dependencies/dlib-0.5.3/LICENSE.txt) |
| document-features | 0.2.12 | MIT OR Apache-2.0 | [notice](licenses/dependencies/document-features-0.2.12/LICENSE-APACHE) |
| downcast-rs | 1.2.1 | MIT/Apache-2.0 | [notice](licenses/dependencies/downcast-rs-1.2.1/LICENSE-APACHE) |
| dpi | 0.1.2 | Apache-2.0 AND MIT | [notice](licenses/dependencies/dpi-0.1.2/LICENSE) |
| either | 1.15.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/either-1.15.0/LICENSE-APACHE) |
| equivalent | 1.0.2 | Apache-2.0 OR MIT | [notice](licenses/dependencies/equivalent-1.0.2/LICENSE-APACHE) |
| errno | 0.3.14 | MIT OR Apache-2.0 | [notice](licenses/dependencies/errno-0.3.14/LICENSE-APACHE) |
| euclid | 0.22.14 | MIT OR Apache-2.0 | [notice](licenses/dependencies/euclid-0.22.14/COPYRIGHT) |
| fdeflate | 0.3.7 | MIT OR Apache-2.0 | [notice](licenses/dependencies/fdeflate-0.3.7/LICENSE-APACHE) |
| find-msvc-tools | 0.1.9 | MIT OR Apache-2.0 | [notice](licenses/dependencies/find-msvc-tools-0.1.9/LICENSE-APACHE) |
| flate2 | 1.1.9 | MIT OR Apache-2.0 | [notice](licenses/dependencies/flate2-1.1.9/LICENSE-APACHE) |
| foldhash | 0.1.5 | Zlib | [notice](licenses/dependencies/foldhash-0.1.5/LICENSE) |
| foldhash | 0.2.0 | Zlib | [notice](licenses/dependencies/foldhash-0.2.0/LICENSE) |
| font-types | 0.11.3 | MIT OR Apache-2.0 | [notice](licenses/dependencies/font-types-0.11.3/LICENSE-APACHE) |
| fontdue | 0.9.3 | MIT OR Apache-2.0 OR Zlib | [notice](licenses/dependencies/fontdue-0.9.3/LICENSE-APACHE) |
| foreign-types | 0.5.0 | MIT/Apache-2.0 | [notice](licenses/dependencies/foreign-types-0.5.0/LICENSE-APACHE) |
| foreign-types-macros | 0.2.3 | MIT/Apache-2.0 | [notice](licenses/dependencies/foreign-types-macros-0.2.3/LICENSE-APACHE) |
| foreign-types-shared | 0.3.1 | MIT/Apache-2.0 | [notice](licenses/dependencies/foreign-types-shared-0.3.1/LICENSE-APACHE) |
| futures-core | 0.3.33 | MIT OR Apache-2.0 | [notice](licenses/dependencies/futures-core-0.3.33/LICENSE-APACHE) |
| futures-intrusive | 0.5.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/futures-intrusive-0.5.0/LICENSE-APACHE) |
| gbdaw-engine | 0.2.0 | MIT | [notice](licenses/dependencies/gbdaw-engine-0.2.0/LICENSE) |
| gbdaw-export-kitaqgb | 0.2.0 | MIT | [notice](licenses/dependencies/gbdaw-export-kitaqgb-0.2.0/LICENSE) |
| gbdaw-kokura | 0.2.0 | MIT | [notice](licenses/dependencies/gbdaw-kokura-0.2.0/LICENSE) |
| gbdaw-model | 0.2.0 | MIT | [notice](licenses/dependencies/gbdaw-model-0.2.0/LICENSE) |
| gethostname | 1.1.0 | Apache-2.0 | [notice](licenses/dependencies/gethostname-1.1.0/LICENSE) |
| getrandom | 0.3.4 | MIT OR Apache-2.0 | [notice](licenses/dependencies/getrandom-0.3.4/LICENSE-APACHE) |
| gl_generator | 0.14.0 | Apache-2.0 | [notice](licenses/dependencies/gl_generator-0.14.0/upstream/LICENSE) |
| glob | 0.3.3 | MIT OR Apache-2.0 | [notice](licenses/dependencies/glob-0.3.3/LICENSE-APACHE) |
| glow | 0.16.0 | MIT OR Apache-2.0 OR Zlib | [notice](licenses/dependencies/glow-0.16.0/LICENSE-APACHE) |
| glutin_wgl_sys | 0.6.1 | Apache-2.0 | [notice](licenses/dependencies/glutin_wgl_sys-0.6.1/LICENSE) |
| gpu-allocator | 0.28.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/gpu-allocator-0.28.0/LICENSE-APACHE) |
| gpu-descriptor | 0.3.2 | MIT OR Apache-2.0 | [notice](licenses/dependencies/gpu-descriptor-0.3.2/upstream/COPYING) |
| gpu-descriptor-types | 0.2.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/gpu-descriptor-types-0.2.0/upstream/COPYING) |
| guillotiere | 0.7.0 | MIT/Apache-2.0 | [notice](licenses/dependencies/guillotiere-0.7.0/LICENSE-APACHE) |
| half | 2.7.1 | MIT OR Apache-2.0 | [notice](licenses/dependencies/half-2.7.1/LICENSE-APACHE) |
| hashbrown | 0.15.5 | MIT OR Apache-2.0 | [notice](licenses/dependencies/hashbrown-0.15.5/LICENSE-APACHE) |
| hashbrown | 0.16.1 | MIT OR Apache-2.0 | [notice](licenses/dependencies/hashbrown-0.16.1/LICENSE-APACHE) |
| hashbrown | 0.17.1 | MIT OR Apache-2.0 | [notice](licenses/dependencies/hashbrown-0.17.1/LICENSE-APACHE) |
| heck | 0.5.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/heck-0.5.0/LICENSE-APACHE) |
| hexf-parse | 0.2.1 | CC0-1.0 | [notice](licenses/dependencies/hexf-parse-0.2.1/CC0-1.0.txt) |
| indexmap | 2.14.0 | Apache-2.0 OR MIT | [notice](licenses/dependencies/indexmap-2.14.0/LICENSE-APACHE) |
| is_terminal_polyfill | 1.70.2 | MIT OR Apache-2.0 | [notice](licenses/dependencies/is_terminal_polyfill-1.70.2/LICENSE-APACHE) |
| itertools | 0.13.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/itertools-0.13.0/LICENSE-APACHE) |
| itoa | 1.0.18 | MIT OR Apache-2.0 | [notice](licenses/dependencies/itoa-1.0.18/LICENSE-APACHE) |
| jobserver | 0.1.34 | MIT OR Apache-2.0 | [notice](licenses/dependencies/jobserver-0.1.34/LICENSE-APACHE) |
| khronos-egl | 6.0.0 | MIT/Apache-2.0 | [notice](licenses/dependencies/khronos-egl-6.0.0/LICENSE-APACHE) |
| khronos_api | 3.1.0 | Apache-2.0 | [notice](licenses/dependencies/khronos_api-3.1.0/upstream/LICENSE) |
| kokura-core | 0.2.0 | MIT | [notice](licenses/dependencies/kokura-core-0.2.0/LICENSE) |
| kurbo | 0.13.1 | Apache-2.0 OR MIT | [notice](licenses/dependencies/kurbo-0.13.1/LICENSE-APACHE) |
| lazy_static | 1.5.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/lazy_static-1.5.0/LICENSE-APACHE) |
| libc | 0.2.189 | MIT OR Apache-2.0 | [notice](licenses/dependencies/libc-0.2.189/LICENSE-APACHE) |
| libloading | 0.8.9 | ISC | [notice](licenses/dependencies/libloading-0.8.9/LICENSE) |
| libm | 0.2.16 | MIT | [notice](licenses/dependencies/libm-0.2.16/LICENSE.txt) |
| linebender_resource_handle | 0.1.1 | Apache-2.0 OR MIT | [notice](licenses/dependencies/linebender_resource_handle-0.1.1/LICENSE-APACHE) |
| linux-raw-sys | 0.12.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | [notice](licenses/dependencies/linux-raw-sys-0.12.1/COPYRIGHT) |
| linux-raw-sys | 0.4.15 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | [notice](licenses/dependencies/linux-raw-sys-0.4.15/COPYRIGHT) |
| litrs | 1.0.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/litrs-1.0.0/LICENSE-APACHE) |
| lock_api | 0.4.14 | MIT OR Apache-2.0 | [notice](licenses/dependencies/lock_api-0.4.14/LICENSE-APACHE) |
| log | 0.4.33 | MIT OR Apache-2.0 | [notice](licenses/dependencies/log-0.4.33/LICENSE-APACHE) |
| mach2 | 0.4.3 | BSD-2-Clause OR MIT OR Apache-2.0 | [notice](licenses/dependencies/mach2-0.4.3/LICENSE-APACHE) |
| malloc_buf | 0.0.6 | MIT | [notice](licenses/dependencies/malloc_buf-0.0.6/LICENSE) |
| matchers | 0.2.0 | MIT | [notice](licenses/dependencies/matchers-0.2.0/LICENSE) |
| memchr | 2.8.3 | Unlicense OR MIT | [notice](licenses/dependencies/memchr-2.8.3/COPYING) |
| memmap2 | 0.9.10 | MIT OR Apache-2.0 | [notice](licenses/dependencies/memmap2-0.9.10/LICENSE-APACHE) |
| metal | 0.33.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/metal-0.33.0/LICENSE-APACHE) |
| minimal-lexical | 0.2.1 | MIT/Apache-2.0 | [notice](licenses/dependencies/minimal-lexical-0.2.1/LICENSE-APACHE) |
| miniz_oxide | 0.8.9 | MIT OR Zlib OR Apache-2.0 | [notice](licenses/dependencies/miniz_oxide-0.8.9/LICENSE) |
| naga | 28.0.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/naga-28.0.0/LICENSE.APACHE) |
| nom | 7.1.3 | MIT | [notice](licenses/dependencies/nom-7.1.3/LICENSE) |
| nu-ansi-term | 0.50.3 | MIT | [notice](licenses/dependencies/nu-ansi-term-0.50.3/LICENSE) |
| num-traits | 0.2.19 | MIT OR Apache-2.0 | [notice](licenses/dependencies/num-traits-0.2.19/LICENSE-APACHE) |
| objc | 0.2.7 | MIT | [notice](licenses/dependencies/objc-0.2.7/LICENSE.txt) |
| objc-sys | 0.3.5 | MIT | [notice](licenses/dependencies/objc-sys-0.3.5/LICENSE.txt) |
| objc2 | 0.5.2 | MIT | [notice](licenses/dependencies/objc2-0.5.2/LICENSE.txt) |
| objc2 | 0.6.4 | MIT | [notice](licenses/dependencies/objc2-0.6.4/LICENSE-MIT-upstream.txt) |
| objc2-app-kit | 0.2.2 | MIT | [notice](licenses/dependencies/objc2-app-kit-0.2.2/LICENSE.txt) |
| objc2-app-kit | 0.3.2 | Zlib OR Apache-2.0 OR MIT | [notice](licenses/dependencies/objc2-app-kit-0.3.2/LICENSE-MIT-upstream.txt) |
| objc2-core-data | 0.2.2 | MIT | [notice](licenses/dependencies/objc2-core-data-0.2.2/LICENSE.txt) |
| objc2-core-foundation | 0.3.2 | Zlib OR Apache-2.0 OR MIT | [notice](licenses/dependencies/objc2-core-foundation-0.3.2/LICENSE-MIT-upstream.txt) |
| objc2-core-image | 0.2.2 | MIT | [notice](licenses/dependencies/objc2-core-image-0.2.2/LICENSE.txt) |
| objc2-encode | 4.1.0 | MIT | [notice](licenses/dependencies/objc2-encode-4.1.0/LICENSE-MIT-upstream.txt) |
| objc2-foundation | 0.2.2 | MIT | [notice](licenses/dependencies/objc2-foundation-0.2.2/src/copying.rs) |
| objc2-foundation | 0.3.2 | MIT | [notice](licenses/dependencies/objc2-foundation-0.3.2/src/copying.rs) |
| objc2-metal | 0.2.2 | MIT | [notice](licenses/dependencies/objc2-metal-0.2.2/LICENSE.txt) |
| objc2-quartz-core | 0.2.2 | MIT | [notice](licenses/dependencies/objc2-quartz-core-0.2.2/LICENSE.txt) |
| once_cell | 1.21.4 | MIT OR Apache-2.0 | [notice](licenses/dependencies/once_cell-1.21.4/LICENSE-APACHE) |
| once_cell_polyfill | 1.70.2 | MIT OR Apache-2.0 | [notice](licenses/dependencies/once_cell_polyfill-1.70.2/LICENSE-APACHE) |
| ordered-float | 5.3.0 | MIT | [notice](licenses/dependencies/ordered-float-5.3.0/LICENSE-MIT) |
| owned_ttf_parser | 0.25.1 | Apache-2.0 | [notice](licenses/dependencies/owned_ttf_parser-0.25.1/LICENSE) |
| parking_lot | 0.12.5 | MIT OR Apache-2.0 | [notice](licenses/dependencies/parking_lot-0.12.5/LICENSE-APACHE) |
| parking_lot_core | 0.9.12 | MIT OR Apache-2.0 | [notice](licenses/dependencies/parking_lot_core-0.9.12/LICENSE-APACHE) |
| paste | 1.0.15 | MIT OR Apache-2.0 | [notice](licenses/dependencies/paste-1.0.15/LICENSE-APACHE) |
| peniko | 0.6.1 | Apache-2.0 OR MIT | [notice](licenses/dependencies/peniko-0.6.1/LICENSE-APACHE) |
| percent-encoding | 2.3.2 | MIT OR Apache-2.0 | [notice](licenses/dependencies/percent-encoding-2.3.2/LICENSE-APACHE) |
| pin-project-lite | 0.2.17 | Apache-2.0 OR MIT | [notice](licenses/dependencies/pin-project-lite-0.2.17/LICENSE-APACHE) |
| pkg-config | 0.3.33 | MIT OR Apache-2.0 | [notice](licenses/dependencies/pkg-config-0.3.33/LICENSE-APACHE) |
| plita | 0.0.1 | MIT | [notice](licenses/dependencies/plita-0.0.1/LICENSE) |
| plita_app | 0.0.1 | MIT | [notice](licenses/dependencies/plita_app-0.0.1/LICENSE) |
| plita_render | 0.0.1 | MIT | [notice](licenses/dependencies/plita_render-0.0.1/LICENSE) |
| plita_ui | 0.0.1 | MIT | [notice](licenses/dependencies/plita_ui-0.0.1/assets/lucide/LICENSE) |
| plita_window | 0.0.1 | MIT | [notice](licenses/dependencies/plita_window-0.0.1/LICENSE) |
| png | 0.18.1 | MIT OR Apache-2.0 | [notice](licenses/dependencies/png-0.18.1/LICENSE-APACHE) |
| polling | 3.11.0 | Apache-2.0 OR MIT | [notice](licenses/dependencies/polling-3.11.0/LICENSE-APACHE) |
| pollster | 0.4.0 | Apache-2.0/MIT | [notice](licenses/dependencies/pollster-0.4.0/LICENSE-APACHE) |
| polycool | 0.4.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/polycool-0.4.0/LICENSE-APACHE) |
| presser | 0.3.1 | MIT OR Apache-2.0 | [notice](licenses/dependencies/presser-0.3.1/LICENSE-APACHE) |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 | [notice](licenses/dependencies/proc-macro2-1.0.107/LICENSE-APACHE) |
| profiling | 1.0.18 | MIT OR Apache-2.0 | [notice](licenses/dependencies/profiling-1.0.18/upstream/LICENSE-APACHE) |
| quick-xml | 0.39.4 | MIT | [notice](licenses/dependencies/quick-xml-0.39.4/LICENSE-MIT.md) |
| quote | 1.0.47 | MIT OR Apache-2.0 | [notice](licenses/dependencies/quote-1.0.47/LICENSE-APACHE) |
| range-alloc | 0.1.5 | MIT OR Apache-2.0 | [notice](licenses/dependencies/range-alloc-0.1.5/LICENSE.APACHE) |
| raw-window-handle | 0.6.2 | MIT OR Apache-2.0 OR Zlib | [notice](licenses/dependencies/raw-window-handle-0.6.2/LICENSE-APACHE.md) |
| read-fonts | 0.37.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/read-fonts-0.37.0/LICENSE-APACHE) |
| regex | 1.12.3 | MIT OR Apache-2.0 | [notice](licenses/dependencies/regex-1.12.3/LICENSE-APACHE) |
| regex-automata | 0.4.16 | MIT OR Apache-2.0 | [notice](licenses/dependencies/regex-automata-0.4.16/LICENSE-APACHE) |
| regex-syntax | 0.8.11 | MIT OR Apache-2.0 | [notice](licenses/dependencies/regex-syntax-0.8.11/LICENSE-APACHE) |
| renderdoc-sys | 1.1.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/renderdoc-sys-1.1.0/LICENSE-APACHE) |
| rfd | 0.17.2 | MIT | [notice](licenses/dependencies/rfd-0.17.2/LICENSE) |
| rodio | 0.19.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/rodio-0.19.0/LICENSE-APACHE) |
| rustc-hash | 1.1.0 | Apache-2.0/MIT | [notice](licenses/dependencies/rustc-hash-1.1.0/LICENSE-APACHE) |
| rustc-hash | 2.1.2 | Apache-2.0 OR MIT | [notice](licenses/dependencies/rustc-hash-2.1.2/LICENSE-APACHE) |
| rustix | 0.38.44 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | [notice](licenses/dependencies/rustix-0.38.44/COPYRIGHT) |
| rustix | 1.1.4 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | [notice](licenses/dependencies/rustix-1.1.4/COPYRIGHT) |
| scoped-tls | 1.0.1 | MIT/Apache-2.0 | [notice](licenses/dependencies/scoped-tls-1.0.1/LICENSE-APACHE) |
| scopeguard | 1.2.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/scopeguard-1.2.0/LICENSE-APACHE) |
| sctk-adwaita | 0.10.1 | MIT | [notice](licenses/dependencies/sctk-adwaita-0.10.1/LICENSE) |
| serde | 1.0.229 | MIT OR Apache-2.0 | [notice](licenses/dependencies/serde-1.0.229/LICENSE-APACHE) |
| serde_core | 1.0.229 | MIT OR Apache-2.0 | [notice](licenses/dependencies/serde_core-1.0.229/LICENSE-APACHE) |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 | [notice](licenses/dependencies/serde_derive-1.0.229/LICENSE-APACHE) |
| serde_json | 1.0.151 | MIT OR Apache-2.0 | [notice](licenses/dependencies/serde_json-1.0.151/LICENSE-APACHE) |
| sharded-slab | 0.1.7 | MIT | [notice](licenses/dependencies/sharded-slab-0.1.7/LICENSE) |
| shlex | 1.3.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/shlex-1.3.0/LICENSE-APACHE) |
| simd-adler32 | 0.3.10 | MIT | [notice](licenses/dependencies/simd-adler32-0.3.10/LICENSE.md) |
| skrifa | 0.40.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/skrifa-0.40.0/LICENSE-APACHE) |
| slab | 0.4.12 | MIT | [notice](licenses/dependencies/slab-0.4.12/LICENSE) |
| smallvec | 1.15.2 | MIT OR Apache-2.0 | [notice](licenses/dependencies/smallvec-1.15.2/LICENSE-APACHE) |
| smithay-client-toolkit | 0.19.2 | MIT | [notice](licenses/dependencies/smithay-client-toolkit-0.19.2/LICENSE.txt) |
| smol_str | 0.2.2 | MIT OR Apache-2.0 | [notice](licenses/dependencies/smol_str-0.2.2/LICENSE-APACHE) |
| spirv | 0.3.0+sdk-1.3.268.0 | Apache-2.0 | [notice](licenses/dependencies/spirv-0.3.0+sdk-1.3.268.0/upstream/LICENSE) |
| static_assertions | 1.1.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/static_assertions-1.1.0/LICENSE-APACHE) |
| strict-num | 0.1.1 | MIT | [notice](licenses/dependencies/strict-num-0.1.1/LICENSE) |
| strsim | 0.11.1 | MIT | [notice](licenses/dependencies/strsim-0.11.1/LICENSE) |
| svg_fmt | 0.4.5 | MIT/Apache-2.0 | [notice](licenses/dependencies/svg_fmt-0.4.5/upstream/LICENSE) |
| syn | 2.0.119 | MIT OR Apache-2.0 | [notice](licenses/dependencies/syn-2.0.119/LICENSE-APACHE) |
| syn | 3.0.3 | MIT OR Apache-2.0 | [notice](licenses/dependencies/syn-3.0.3/LICENSE-APACHE) |
| thiserror | 1.0.69 | MIT OR Apache-2.0 | [notice](licenses/dependencies/thiserror-1.0.69/LICENSE-APACHE) |
| thiserror | 2.0.19 | MIT OR Apache-2.0 | [notice](licenses/dependencies/thiserror-2.0.19/LICENSE-APACHE) |
| thiserror-impl | 1.0.69 | MIT OR Apache-2.0 | [notice](licenses/dependencies/thiserror-impl-1.0.69/LICENSE-APACHE) |
| thiserror-impl | 2.0.19 | MIT OR Apache-2.0 | [notice](licenses/dependencies/thiserror-impl-2.0.19/LICENSE-APACHE) |
| thread_local | 1.1.10 | MIT OR Apache-2.0 | [notice](licenses/dependencies/thread_local-1.1.10/LICENSE-APACHE) |
| tiny-skia | 0.11.4 | BSD-3-Clause | [notice](licenses/dependencies/tiny-skia-0.11.4/LICENSE) |
| tiny-skia-path | 0.11.4 | BSD-3-Clause | [notice](licenses/dependencies/tiny-skia-path-0.11.4/LICENSE) |
| tracing | 0.1.44 | MIT | [notice](licenses/dependencies/tracing-0.1.44/LICENSE) |
| tracing-attributes | 0.1.31 | MIT | [notice](licenses/dependencies/tracing-attributes-0.1.31/LICENSE) |
| tracing-core | 0.1.36 | MIT | [notice](licenses/dependencies/tracing-core-0.1.36/LICENSE) |
| tracing-log | 0.2.0 | MIT | [notice](licenses/dependencies/tracing-log-0.2.0/LICENSE) |
| tracing-subscriber | 0.3.23 | MIT | [notice](licenses/dependencies/tracing-subscriber-0.3.23/LICENSE) |
| ttf-parser | 0.21.1 | MIT OR Apache-2.0 | [notice](licenses/dependencies/ttf-parser-0.21.1/LICENSE-APACHE) |
| ttf-parser | 0.25.1 | MIT OR Apache-2.0 | [notice](licenses/dependencies/ttf-parser-0.25.1/LICENSE-APACHE) |
| unicode-ident | 1.0.24 | (MIT OR Apache-2.0) AND Unicode-3.0 | [notice](licenses/dependencies/unicode-ident-1.0.24/LICENSE-APACHE) |
| unicode-segmentation | 1.13.3 | MIT OR Apache-2.0 | [notice](licenses/dependencies/unicode-segmentation-1.13.3/COPYRIGHT) |
| unicode-width | 0.2.2 | MIT OR Apache-2.0 | [notice](licenses/dependencies/unicode-width-0.2.2/COPYRIGHT) |
| utf8parse | 0.2.2 | Apache-2.0 OR MIT | [notice](licenses/dependencies/utf8parse-0.2.2/LICENSE-APACHE) |
| vello | 0.8.0 | Apache-2.0 OR MIT | [notice](licenses/dependencies/vello-0.8.0/LICENSE-APACHE) |
| vello_encoding | 0.8.0 | Apache-2.0 OR MIT | [notice](licenses/dependencies/vello_encoding-0.8.0/LICENSE-APACHE) |
| vello_shaders | 0.8.0 | Apache-2.0 OR MIT | [notice](licenses/dependencies/vello_shaders-0.8.0/LICENSE-APACHE) |
| version_check | 0.9.5 | MIT/Apache-2.0 | [notice](licenses/dependencies/version_check-0.9.5/LICENSE-APACHE) |
| wayland-backend | 0.3.15 | MIT | [notice](licenses/dependencies/wayland-backend-0.3.15/LICENSE.txt) |
| wayland-client | 0.31.14 | MIT | [notice](licenses/dependencies/wayland-client-0.31.14/LICENSE.txt) |
| wayland-csd-frame | 0.3.0 | MIT | [notice](licenses/dependencies/wayland-csd-frame-0.3.0/LICENSE) |
| wayland-cursor | 0.31.14 | MIT | [notice](licenses/dependencies/wayland-cursor-0.31.14/LICENSE.txt) |
| wayland-protocols | 0.32.12 | MIT | [notice](licenses/dependencies/wayland-protocols-0.32.12/LICENSE.txt) |
| wayland-protocols-plasma | 0.3.12 | MIT | [notice](licenses/dependencies/wayland-protocols-plasma-0.3.12/LICENSE.txt) |
| wayland-protocols-wlr | 0.3.12 | MIT | [notice](licenses/dependencies/wayland-protocols-wlr-0.3.12/LICENSE.txt) |
| wayland-scanner | 0.31.10 | MIT | [notice](licenses/dependencies/wayland-scanner-0.31.10/LICENSE.txt) |
| wayland-sys | 0.31.11 | MIT | [notice](licenses/dependencies/wayland-sys-0.31.11/LICENSE.txt) |
| wgpu | 28.0.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/wgpu-28.0.0/LICENSE.APACHE) |
| wgpu-core | 28.0.1 | MIT OR Apache-2.0 | [notice](licenses/dependencies/wgpu-core-28.0.1/LICENSE.APACHE) |
| wgpu-core-deps-apple | 28.0.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/wgpu-core-deps-apple-28.0.0/LICENSE.APACHE) |
| wgpu-core-deps-windows-linux-android | 28.0.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/wgpu-core-deps-windows-linux-android-28.0.0/LICENSE.APACHE) |
| wgpu-hal | 28.0.1 | MIT OR Apache-2.0 | [notice](licenses/dependencies/wgpu-hal-28.0.1/LICENSE.APACHE) |
| wgpu-types | 28.0.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/wgpu-types-28.0.0/LICENSE.APACHE) |
| windows | 0.54.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/windows-0.54.0/license-apache-2.0) |
| windows | 0.62.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/windows-0.62.0/license-apache-2.0) |
| windows-collections | 0.3.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/windows-collections-0.3.0/license-apache-2.0) |
| windows-core | 0.54.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/windows-core-0.54.0/license-apache-2.0) |
| windows-core | 0.62.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/windows-core-0.62.0/license-apache-2.0) |
| windows-future | 0.3.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/windows-future-0.3.0/license-apache-2.0) |
| windows-implement | 0.60.2 | MIT OR Apache-2.0 | [notice](licenses/dependencies/windows-implement-0.60.2/license-apache-2.0) |
| windows-interface | 0.59.3 | MIT OR Apache-2.0 | [notice](licenses/dependencies/windows-interface-0.59.3/license-apache-2.0) |
| windows-link | 0.2.1 | MIT OR Apache-2.0 | [notice](licenses/dependencies/windows-link-0.2.1/license-apache-2.0) |
| windows-numerics | 0.3.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/windows-numerics-0.3.0/license-apache-2.0) |
| windows-result | 0.1.2 | MIT OR Apache-2.0 | [notice](licenses/dependencies/windows-result-0.1.2/license-apache-2.0) |
| windows-result | 0.4.1 | MIT OR Apache-2.0 | [notice](licenses/dependencies/windows-result-0.4.1/license-apache-2.0) |
| windows-strings | 0.5.1 | MIT OR Apache-2.0 | [notice](licenses/dependencies/windows-strings-0.5.1/license-apache-2.0) |
| windows-sys | 0.52.0 | MIT OR Apache-2.0 | [notice](licenses/dependencies/windows-sys-0.52.0/license-apache-2.0) |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 | [notice](licenses/dependencies/windows-sys-0.61.2/license-apache-2.0) |
| windows-targets | 0.52.6 | MIT OR Apache-2.0 | [notice](licenses/dependencies/windows-targets-0.52.6/license-apache-2.0) |
| windows-threading | 0.2.1 | MIT OR Apache-2.0 | [notice](licenses/dependencies/windows-threading-0.2.1/license-apache-2.0) |
| windows_x86_64_msvc | 0.52.6 | MIT OR Apache-2.0 | [notice](licenses/dependencies/windows_x86_64_msvc-0.52.6/license-apache-2.0) |
| winit | 0.30.13 | Apache-2.0 | [notice](licenses/dependencies/winit-0.30.13/LICENSE) |
| x11-dl | 2.21.0 | MIT | [notice](licenses/dependencies/x11-dl-2.21.0/LICENSE-MIT) |
| x11rb | 0.13.2 | MIT OR Apache-2.0 | [notice](licenses/dependencies/x11rb-0.13.2/LICENSE-APACHE) |
| x11rb-protocol | 0.13.2 | MIT OR Apache-2.0 | [notice](licenses/dependencies/x11rb-protocol-0.13.2/LICENSE-APACHE) |
| xcursor | 0.3.10 | MIT | [notice](licenses/dependencies/xcursor-0.3.10/LICENSE) |
| xkbcommon-dl | 0.4.2 | MIT | [notice](licenses/dependencies/xkbcommon-dl-0.4.2/LICENSE) |
| xkeysym | 0.2.1 | MIT OR Apache-2.0 OR Zlib | [notice](licenses/dependencies/xkeysym-0.2.1/LICENSE-APACHE) |
| xml-rs | 0.8.28 | MIT | [notice](licenses/dependencies/xml-rs-0.8.28/LICENSE) |
| zerocopy | 0.8.55 | BSD-2-Clause OR Apache-2.0 OR MIT | [notice](licenses/dependencies/zerocopy-0.8.55/LICENSE-APACHE) |
| zerocopy-derive | 0.8.55 | BSD-2-Clause OR Apache-2.0 OR MIT | [notice](licenses/dependencies/zerocopy-derive-0.8.55/LICENSE-APACHE) |
| zmij | 1.0.23 | MIT | [notice](licenses/dependencies/zmij-1.0.23/LICENSE-MIT) |

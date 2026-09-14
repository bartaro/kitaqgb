# KITAQGB Libraries

`wire3d_dmg` is a monochrome wireframe renderer for Game Boy. Select 128 × 96 with `wire3d_dmg_96.c`, or 128 × 120 with `wire3d_dmg.c`, and use `Wire3DDMG_*`. The old `wire3d` and `dmg3d` files remain compatibility entries for those respective profiles. Compile only one entry. `wire3d_cgb` retains its name and remains the separate color renderer.

[Shared renderer guide](wire3d_dmg_guide.md) / [日本語](wire3d_dmg_guide_ja.md)

<!-- manual-language-links:start -->
| Language / 言語 | HTML |
| --- | --- |
| English | [KITAQGB Library](https://bartaro.github.io/kitaq-docs/en/gb-library.html) |
| 日本語 | [KITAQGB Library](https://bartaro.github.io/kitaq-docs/gb-library.html) |
| 한국어 | [KITAQGB Library](https://bartaro.github.io/kitaq-docs/ko/gb-library.html) |
| 简体中文 | [KITAQGB Library](https://bartaro.github.io/kitaq-docs/zh-CN/gb-library.html) |
| 繁體中文 | [KITAQGB Library](https://bartaro.github.io/kitaq-docs/zh-TW/gb-library.html) |
| Español | [KITAQGB Library](https://bartaro.github.io/kitaq-docs/es/gb-library.html) |
| Português (Brasil) | [KITAQGB Library](https://bartaro.github.io/kitaq-docs/pt/gb-library.html) |
| Français | [KITAQGB Library](https://bartaro.github.io/kitaq-docs/fr/gb-library.html) |
| Deutsch | [KITAQGB Library](https://bartaro.github.io/kitaq-docs/de/gb-library.html) |
<!-- manual-language-links:end -->

[English](README.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Español](README.es.md) | [Português (Brasil)](README.pt-BR.md) | [Français](README.fr.md) | [Deutsch](README.de.md)


This folder contains three kinds of files:

- Public API
  - Normal reusable libraries intended to be included and linked from game projects.
- Helper units
  - Optional support units, register-definition stubs, or placeholder source files that are not peer public APIs by themselves.
- Reference docs
  - Documentation only; not linked into ROM builds.

## Classification

### Public API

| Files | Purpose | Normal use |
| --- | --- | --- |
| `physics2d.h` / `physics2d.c` | 2D AABB bodies, gravity integration, iterative contact solver. | Include `physics2d.h` and compile `physics2d.c` when used. |
| `physics2d_circle.h` / `physics2d_circle.c` | 2D circle-body physics for ball-style games. | Include the header and compile the source when used. |
| `physics3d.h` / `physics3d.c` | 3D AABB physics with acceleration, mass-weighted bounce, break flags, plus `kq3d_dot_q8_8()`. | Include the header and compile the source when used. |
| `wire3d.h` / `wire3d.c` | Fixed-point wireframe 3D renderer with WRAM staging, hidden-line models, and scene occlusion masks. | Include `wire3d.h` and compile `wire3d.c` when used. |
| `dmg3d.h` / `dmg3d.c` | DMG staged wireframe renderer with a 128x120 D000 staging surface, fixed-order inline-assembly line drawing, and STAT-gated D000-to-8900 transfer. | Include `dmg3d.h` and compile `dmg3d.c` when a staged 1bpp wireframe pipeline is needed. |
| `wire3d_cgb.h` / `wire3d_cgb.c` | CGB-only 8MHz color wireframe renderer with 2bpp WRAM staging, hidden-line/scene occlusion, clipped ASM drawing, and tear-free HBlank DMA presentation. | Include `wire3d_cgb.h` and compile `wire3d_cgb.c` with a CGB-only ROM build. |
| `system.h` / `system.c` | Small GB runtime base: init, frame count, VBlank wait, cooperative VBlank callback, DI/EI wrappers. | Include `system.h` and compile `system.c` for frame-paced game loops. |
| `input.h` / `input.c` | Frame input state with down/pressed/released/repeat helpers. | Include `input.h` and compile `input.c` for menu, action, puzzle, and SLG controls. |
| `vram.h` / `vram.c` | PPU-safe VRAM command queue for BG tile writes, rect fills, map blocks, memcpy, and memset. | Queue updates during gameplay and call `vram_flush()` or `vram_flush_now()` during a safe window. |
| `sprite.h` / `sprite.c` | Shadow OAM, sprite allocation, metasprite drawing, animation stepping, OAM DMA flush, and scanline-overflow checks. | Include `sprite.h` and compile `sprite.c` for OBJ-based rendering. |
| `fixed.h` / `fixed.c` | Q8.8 fixed-point helpers, `Vec2`, `KQRect`, clamp/min/max/lerp, and basic rectangle tests. | Include `fixed.h` and compile `fixed.c` for movement, physics, camera, and AI scoring helpers. |
| `scene.h` / `scene.c` | Lightweight title/game/pause style scene table and change/update/draw dispatcher. | Include `scene.h` and compile `scene.c` when structuring game-state flow. |
| `entity.h` / `entity.c` | Fixed-array object pool for up to `ENTITY_MAX` small game entities. | Include `entity.h` and compile `entity.c`; callbacks receive an entity id and can call `entity_get(id)`. |
| `danmaku.h` / `danmaku.c` | 96-bullet fixed-point pool, 32-direction fans, hit/graze events, and a CGB BG tile compositor independent of OAM limits. | Include `danmaku.h` and compile `danmaku.c`; see `danmaku_guide.md` and the complete `ressen_gbc` game. |
| `bank.h` / `bank.c` | Far data, far pointer, far call, and simple MBC bank-switch helpers over compiler intrinsics. | Include `bank.h` and compile `bank.c` for banked data access wrappers. |
| `asset.h` / `asset.c` | Small asset-id descriptor table and raw/tile load helpers. | Include `asset.h` and compile `asset.c`; generated `assets.h/c/json` can target this shape later. |
| `debug.h` / `debug.c` | Thin ROM-side trace/assert/mark buffer for KOKURA or emulator-side inspection. | Include `debug.h` and compile `debug.c`; keep heavy profiling outside the ROM. |
| `chain.h` / `chain.c` | Coordinate-history ring buffer for snake, rope, train, or joint-sprite style objects. | Include `chain.h` and compile `chain.c` for segment-history movement. |
| `cgb_tile.h` | Public declarations for compiler-intrinsic CGB tile/attr helpers. | Include from game code when using `__settile...` / CGB tile helpers. |
| `cgb_palette.h` / `cgb_palette.c` | High-level CGB BG/OBJ palette helper layer. | Include the header and compile the source when used. |
| `scroll.h` / `scroll.c` | Scroll and split-table helpers built on compiler intrinsics. | Include the header and compile the source when using `Scroll_*` wrappers. |
| `raster.h` / `raster.c` | Raster scroll band builders plus structured per-scanline X-warp profiles. | Include `raster.h` and compile `raster.c` with `scroll.c` when using `Raster_*` helpers. |
| `camera.h` / `camera.c` | 8.8 fixed-point camera helper built on `scroll.*`, plus simple global camera wrappers and world/screen conversion. | Include the header and compile the source when used. |
| `audio.h` / `audio.c` | Shared Game Boy audio driver with music/SFX/pan/wave/fade helpers and a 68-note range through stream note 67 (`G6`). | Include `audio.h` and compile `audio.c` for audio-enabled projects. |
| `audio_vblank.h` / `audio_vblank.c` | VBlank IRQ BGM driver with the same 68-note range, a 16-record WRAM queue for banked songs, and an optional per-frame hook. | Keep direct-pointer songs in fixed bank 0, or refill the queue from banked code; patch vector `0x0040` with `scripts/patch_gb_vblank_irq.ps1`. |
| `link.h` / `link.c` | Shared serial byte-transfer layer for link cable projects, plus cooperative logical `Link4_*` helpers. | Include `link.h` and compile `link.c` for link-enabled projects. |
| `link_packet.c` | Optional packet-layer extension on top of `link.c`, including per-peer `Link4_*` packet mailboxes. | Compile together with `link.c` only if packet send/read helpers are needed. |
| `link_dmg07.h` / `link_dmg07.c` | Polling external-clock driver for the physical Nintendo DMG-07 Four Player Adapter. | Compile with `link_hwregs_gb.c`; this is separate from the logical `Link4_*` API. |
| `rpg.h` | Shared RPG / ADV / SLG declarations and low-level intrinsic declarations. | Include from game code when using the RPG / ADV / SLG helper family. |
| `rng.c` | `rng8`, `rng16`, `rand_range`, `weighted_choice`, plus `rng_seed`, `rng_next8`, `rng_next16`, `rng_range`, and `rng_chance`. | Compile when using random helpers from `rpg.h`. |
| `flags.c` | 2048-flag bitset helpers and quest-state storage. | Compile when using flag/quest helpers from `rpg.h`. |
| `rle.c` | Simple `[count][value]` RLE decode helpers for RAM and far ROM. | Compile when using `rle_decode*` helpers from `rpg.h`. |
| `text.c` | Tile-string text window, page wait, choice UI helpers, direct XY text, numeric print helpers, and clear/window aliases. | Compile when using text helpers from `rpg.h`. |
| `menu.c` | Vertical menu wrappers, a minimal inventory menu, and a non-blocking menu state API. | Compile when using menu helpers from `rpg.h`. |
| `script.c` | Minimal script-bytecode runner for RPG / ADV flows. | Compile when using script helpers from `rpg.h`. |
| `map.c` | Packed map loader with collision, trigger, camera helpers, and optional 16x16 metatile map support. | Compile when using map helpers from `rpg.h`. |
| `save.c` | MBC5-style SRAM save/load/check/clear helpers with header, version, length, and checksum. | Compile when using save helpers from `rpg.h`. |
| `slg_unit.c` | SLG move/attack range helpers. | Compile when using tactical unit helpers from `rpg.h`. |
| `slg_path.c` | BFS path finding and move-cost flood fill for SLG helpers. | Compile when using tactical path helpers from `rpg.h`. |
| `slg.h` / `slg_board.c` | Generic board, move-list, and undo-stack helpers for board-game or tactical systems. | Include `slg.h` and compile `slg_board.c`; keep game-specific evaluation outside this helper. |

### Helper Units

| Files | Purpose | Notes |
| --- | --- | --- |
| `audio_hwregs_gb.c` | Minimal APU and wave-RAM register declarations. | Use only when the project does not already define those registers via another hwregs unit. |
| `link_hwregs_gb.c` | Minimal `SB` / `SC` / `IF` / `IE` register declarations for link projects. | Use only when the project does not already define the serial registers elsewhere. |
| `cgb_tile.c` | Intentionally empty compilation unit for the CGB tile helper family. | The real public surface is `cgb_tile.h`; compiling this file is harmless but not required for logic. |
| `math.c` | ROM sine table data (`MATH_SIN`). | Present in `lib/`, but not yet documented as a stable public library API. Treat as a project helper/data unit for now. |

### Reference Docs

| Files | Purpose |
| --- | --- |
| `README.md` | This overview and build-usage note. |
| `wire3d_guide_ja.md` | Japanese quick-start guide for the wireframe 3D renderer. |
| `dmg3d_guide_ja.md` | Japanese quick-start guide for the DMG staged wireframe renderer. |
| `wire3d_cgb_guide.md` | Quick-start guide for the CGB-only color wireframe renderer. |
| `physics_guide.html` | English physics usage guide. |
| `physics_guide_ja.html` | Japanese physics usage guide. |

## Build usage

Several commands below refer to historical development demos that are not
included in this public repository, such as `wire3d_cube_demo.c`. They are
build templates requiring the named sources. For the bundled beginner
programs, use `../examples/build.ps1` and the HTML manual. The short
`kitaqgb` command assumes the executable is available on PATH.

Compile your game source together with the library source files:

```powershell
kitaqgb hwregs.c lib/audio.c main.c lib/physics2d.c lib/physics2d_circle.c lib/physics3d.c lib/cgb_palette.c lib/scroll.c lib/camera.c -I lib -o game.gb --profile=dev
```

Wireframe 3D projects compile the renderer source with the game source:

```powershell
.\kitaqgb.exe lib/wire3d.c examples/wire3d_minimal.c -I lib -o examples/wire3d_minimal.gb --profile=dev --stack-bank=fixed --rst-disable --no-disasm
```

The supplied `examples/wire3d_minimal.c` initializes every model field and rotates a cube in the 128x96 viewport. It needs no external graphics or font assets.

DMG projects with a 128x120 viewport can use the 120-line compatibility entry `dmg3d.*`:

```powershell
.\kitaqgb.exe lib/dmg3d.c examples/dmg3d_minimal.c -I lib -o examples/dmg3d_minimal.gb --profile=dev --stack-bank=fixed --rst-disable --no-disasm
```

`DMG3D_Init()` configures a 128x120 visible wire surface using the D000 WRAM stage and tile uploads starting at 0x8900. `DMG3D_BeginFrame()` only resets occlusion state; uploads consume and clear the pixels. `DMG3D_EndFrame()` waits for VBlank and then polls STAT while transferring, potentially continuing beyond VBlank. The supplied `examples/dmg3d_minimal.c` redraws a cross each frame with dirty transfer enabled. Auxiliary transfer is separate and shares source storage with the main stage.

CGB-only color wireframe projects use the separate `wire3d_cgb.*` renderer:

```powershell
kitaqgb lib/wire3d_cgb.c examples/wire3d_cgb_color_demo.c -I lib -o examples/wire3d_cgb_color_demo.gbc --profile=dev --stack-bank=fixed --rst-disable --cgb=cgb_only --rom-title=CGBWIRE3D
```

`Wire3DCGB_Init()` switches CGB hardware to double speed, configures a 128x96 2bpp BG wire surface, and installs the default four-entry BG palette. Both frame API pairs use the tear-free presenter: the 3072-byte stage at `0xD300-0xDEFF` is sent to the inactive VRAM tile bank with HBlank DMA and presented during VBlank without per-frame LCDC changes. The `Fast` pair omits the normal end-frame BG queue check. Use `Wire3DCGB_SetPaletteRGB15()` and `Wire3DCGB_SetLineColor()` / `Wire3DCGB_Draw*Color()` to draw colored wireframe lines.

For CAD-generated direction LODs, `Wire3DCGB_DrawMaskedModel2D()` accepts preprojected signed vertex offsets and a packed visible-edge mask. Its edge walk and assembly rasterizer stay in renderer bank 4, so one model draw does not perform a cross-bank call for every line.

Sparse games that also use shadow OAM can call `sprite_flush_oam()` followed by `Wire3DCGB_EndFrameSparseNow()` after entering VBlank. This skips an initial wait for a fresh VBlank, but DMA and presentation can still wait depending on the dirty range and current scanline. It does not guarantee that all work finishes in that same VBlank.

For CGB lines, use colors 1, 2 and 3. Normal 128 × 96 lines combine color bits, so overlapping colors 1 and 2 become color 3. Color 0 does not erase a line. Clear the frame or use the dedicated erasing functions. Normal `Wire3DCGB_DrawLine2D` and model drawing do not record sparse upload bounds: use `Wire3DCGB_DrawLineClipped2D` for lines that need this tracking, or call `Wire3DCGB_InvalidateFrameHistory` to include the entire viewport in the next sparse upload.

The 160 × 144 mode allocates at most 127 tiles per frame. An allocation failure or an out-of-range coordinate in its fast line path sets `Wire3DCGB_GetFullScreenOverflow()` and suppresses further pixel writes until the next frame reset. Keep vertices within the selected viewport. Triangle-mask padding stops at X=127 in 128 × 96 mode and X=159 in full-screen mode. Follow the API notes for WRAM bank mapping, especially when using full-screen or FastMap functions.

See the [CGB triangle-mask boundary regression](../tests/library/wire3d_cgb_mask_bounds.c) for a complete program that checks both viewport modes.

The historical development example `examples/wire3d_cgb_hiddenline_demo.c` is the interactive hidden-line check. `START` cycles the visible count from one through three, `B` selects an object, the D-pad moves it on X/Y, `A`+Up/Down moves it on Z, `A`+Left/Right rotates Z, and `SELECT`+D-pad rotates X/Y in 22.5-degree steps. Hidden-line and inter-object occlusion stay enabled, and colliding bodies push apart.

RPG / ADV / SLG helper build examples:

```powershell
kitaqgb examples/example_rpg_text.c lib/text.c lib/menu.c -I lib -o text.gb --profile=dev
kitaqgb examples/example_adv_script.c lib/text.c lib/flags.c lib/script.c -I lib -o script.gb --profile=dev
kitaqgb examples/example_slg_cursor.c lib/map.c lib/slg_unit.c lib/slg_path.c -I lib -o slg.gb --profile=dev
```

Standard runtime smoke build:

```powershell
kitaqgb lib/text.c lib/menu.c lib/map.c lib/scroll.c lib/camera.c lib/rng.c lib/save.c lib/system.c lib/input.c lib/vram.c lib/sprite.c lib/fixed.c lib/scene.c lib/entity.c lib/bank.c lib/asset.c lib/debug.c lib/chain.c lib/physics2d.c lib/slg_board.c examples/standard_library_smoke.c -I lib -o examples/standard_library_smoke.gb --profile=dev --rom-title=STDLIBSMK --no-disasm
```

Serial-link projects should compile the serial hwregs unit before the link core:

```powershell
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c main.c -I lib -o game.gb --profile=dev
```

Cooperative host-selected logical 4-player projects use the same files. The host uses
`Link4_InitHost(slot_count)` and selects a target peer with `Link4_SelectPeer()`
or `Link4_SendPacketTo()`, while peers use `Link4_InitPeer(local_slot, slot_count)`
and communicate with host slot `0`.

The example peers can also be built directly as ready-made slot wrappers:

```powershell
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c examples/link4_demo_peer_slot1.c -I lib -o peer1.gb --profile=dev
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c examples/link4_demo_peer_slot2.c -I lib -o peer2.gb --profile=dev
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c examples/link4_demo_peer_slot3.c -I lib -o peer3.gb --profile=dev
```

Physical DMG-07 projects use the dedicated polling driver instead:

```powershell
kitaqgb lib/link_hwregs_gb.c lib/link_dmg07.c main.c -I lib -o dmg07.gb --profile=dev
```

Call `LinkDmg07_Poll()` continuously; the adapter's bytes are much closer
together than one video frame. Call `LinkDmg07_TickFrame()` once per VBlank for
the saturating silence/handshake timeout counters. The driver always arms
external-clock `SC=$80`, replies to pings with `88 88 RATE 01`, and allows only
physical player 1 to request transmission with `AA AA AA AA`. After all consoles
observe `CC CC CC CC`, each four-byte broadcast contains one byte per physical
slot. The adapter broadcasts data one packet after submission, so the driver
discards the initial undefined packet and exposes sent/received sequence values.
`LinkDmg07_RequestRestart()` waits for the next packet boundary, sends an aligned
`FF FF FF FF`, and stops as soon as the adapter's complete all-FF indicator is
received. A transfer-phase silence timeout schedules this restart automatically
without discarding the current four-byte position, so resumed adapter clocks can
complete the current packet and enter recovery safely.

Then include headers from your game code:

```c
#include "physics2d.h"
#include "physics2d_circle.h"
#include "physics3d.h"
#include "wire3d.h"
#include "cgb_tile.h"
#include "cgb_palette.h"
#include "scroll.h"
#include "raster.h"
#include "camera.h"
#include "audio.h"
#include "audio_vblank.h"
#include "system.h"
#include "input.h"
#include "vram.h"
#include "sprite.h"
#include "fixed.h"
#include "scene.h"
#include "entity.h"
#include "bank.h"
#include "asset.h"
#include "debug.h"
#include "chain.h"
#include "slg.h"
```

## Notes

The final eight audio note indices currently reuse the preceding octave's frequencies; a 68-index range is not a 68-distinct-pitch guarantee.

- `inv_mass_q8 == 0` means a static body.
- `Wire3D_Init()` owns a 128x96 BG wireframe surface, WRAM stage bytes starting at `0xD000`, and tile data starting at `0x8900`; build Wire3D projects with `--stack-bank=fixed`.
- `Wire3D_BeginFrame()` clears the WRAM stage, while `Wire3D_EndFrame()` waits for VBlank and uses a STAT-gated burst copy into VRAM.
- Wire3D angles are 16-step values, and the initial model path supports up to `WIRE3D_MODEL_VERTEX_LIMIT` vertices per model.
- Use `Wire3D_DrawScene()` when multiple solid-ish wire objects overlap; it draws near objects first and accumulates their visible face masks so farther lines are skipped conservatively.
- `Wire3DCGB_Init()` is CGB-only and calls the KEY1/STOP double-speed switch. Build CGB projects with `--cgb=cgb_only`; do not mix `wire3d_cgb.*` with the DMG-compatible `wire3d.*` renderer in the same ROM.
- The standard and `Fast` frame pairs keep the old frame visible until the inactive VRAM tile bank is complete. Prefer the `Fast` pair when the scene does not use queued BG tile writes.
- Compile the file that declares `NR10..NR52` and `WAVE0..WAVE15` before `lib/audio.c`.
- `lib/audio_hwregs_gb.c` is a ready-made option for that, but do not compile it alongside another file that already declares the same sound registers.
- `cgb_tile.h` exposes compiler intrinsics directly; `lib/cgb_tile.c` is only a placeholder unit and can be omitted from normal builds.
- `cgb_palette.h` uses `cgb_*` as the public API surface.
- Call `Audio_SetMusicEnabled()` / `Audio_SetSfxEnabled()` whenever your menu or settings change.
- `Audio_PlaySFX()` captures the current visible ROM bank. Use `Audio_PlaySFXBanked(bank, sfx, priority)` when the SFX data bank is known explicitly.
- Music stream command IDs keep the legacy encoding for `AUDIO_CMD_NOTE` / `AUDIO_CMD_SET_INST`: `0=CH1`, `1=CH2`, `2=CH4`, `3=CH3`.
- Use `Audio_LoadCustomWave()` with 16 bytes holding 32 packed 4-bit samples if you want a project-specific CH3 waveform.
- `Audio_FadeToMasterVolume()` advances from `Audio_Update()`, so call `Audio_Update()` each frame during fades.
- `audio_vblank.c` owns the VBlank IRQ vector symbol `__kq_vblank_vector`. Its BGM stream format is five bytes per event: `delay, ch2_note, ch1_note, ch3_note, ch4_noise_param`; use `AUDIO_VBLANK_REST`, `AUDIO_VBLANK_LOOP`, and `AUDIO_VBLANK_END`.
- Direct-pointer VBlank BGM songs must be fixed-bank data; queue mode can be replenished from banked songs. After linking a ROM with `lib/audio_vblank.c`, run `scripts/patch_gb_vblank_irq.ps1 <rom> <map>` so vector `0x0040` jumps to the ISR and the ROM checksums are refreshed.
- Do not combine `audio_vblank.c` with another library or game stub that also owns VBlank vector `0x0040` unless you add a shared IRQ dispatcher.
- `Scroll_SplitCommit()` auto-enables IE bits `0x01 | 0x02` and uses compiler-owned VBlank/STAT handlers for split playback.
- Split helpers currently reserve vectors `0x0040` and `0x0048` for that build, so do not combine them with a separate custom VBlank/STAT stub yet.
- Compile the file that declares `SB`, `SC`, `IF`, and `IE` before `lib/link.c` / `lib/link_packet.c` or `lib/link_dmg07.c`.
- `lib/link_hwregs_gb.c` is a ready-made option for that, but do not compile it alongside another file that already declares the same serial registers.
- The link library does not claim serial vector `0x0058`; call `Link_OnSerialIRQ()` from your own IRQ stub or dispatcher if you enable interrupt mode.
- The packet layer is intentionally one-deep and is best driven from a frame-paced main loop.
- `Link4_*` models a cooperative host-selected 4-player adapter: only one peer is active on the wire at a time, and the host must rotate peers intentionally.
- `Link4_TryReadByteFrom()` / `Link4_HasPacketFrom()` expose per-peer mailboxes so host code can poll several peers without losing attribution.
- `Link_ReadPacket()` remains a legacy "latest packet" view; use `Link4_ReadPacketFrom()` in 4-player flows.
- `Link4_*` does not implement the Nintendo DMG-07 electrical/protocol behavior. Use `link_dmg07.c` for the physical accessory and do not compile it together with `link.c` in the same ROM.
- DMG-07 `GetConnectedMask()` is normalized to bits 0..3 for physical players 1..4. During transmission it remains the last ping result; peer membership can only be refreshed in ping phase.
- A pending DMG-07 restart cannot make progress while adapter clocks are absent. Recovery traffic is discarded rather than exposed as sequencer data. If the accessory was power-cycled into a different phase instead of merely pausing its clocks, reinitialize the driver/session explicitly.
- These libraries solve linear position/velocity only (no angular dynamics).
- Keep body counts small on GB-class hardware (for example 8-24 active bodies).
- For stricter gameplay behavior, tune gravity, max speed, and solver iteration count per world.
- For billiards-like games, prefer `physics2d_circle.*` over the AABB library.
- Do not add separate `random`, `collision`, `ui`, `tilemap`, `dialog`, `board_game`, or `simple_physics` libraries for the current shape. Use `rng`, `physics2d`, `text`/`menu`, `map`, `script`, `slg`, and `physics2d` respectively.
- `scene.c` and `entity.c` avoid pointer-sized function-pointer arguments because the current KITAQGB function-pointer call path is most reliable with no-arg calls or one-byte ids.

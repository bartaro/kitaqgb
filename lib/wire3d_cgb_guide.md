# Wire3DCGB Guide

`wire3d_cgb.h` / `wire3d_cgb.c` provide a CGB-only color 3D wireframe renderer. It does not keep DMG compatibility. `Wire3DCGB_Init()` switches CGB hardware to double speed, so the renderer is built around the 8MHz CGB path.

## Build

```powershell
kitaqgb lib/wire3d_cgb.c examples/wire3d_cgb_color_demo.c -I lib -o examples/wire3d_cgb_color_demo.gbc --profile=dev --stack-bank=fixed --rst-disable --cgb=cgb_only --rom-title=CGBWIRE3D
```

The included demo can also be built with:

```powershell
.\examples\wire3d_cgb_color_demo_build.ps1
```

## Basic Loop

```c
#include "wire3d_cgb.h"

void main()
{
    Wire3DCGB_Init();
    Wire3DCGB_SetPaletteRGB15(
        WIRE3DCGB_RGB15(0, 0, 2),
        WIRE3DCGB_RGB15(4, 18, 31),
        WIRE3DCGB_RGB15(28, 31, 28),
        WIRE3DCGB_RGB15(31, 12, 3));

    while (1)
    {
        Wire3DCGB_BeginFrameFast();
        Wire3DCGB_SetCamera(0, 8, -28, 0, 0, 0);
        Wire3DCGB_DrawLine3DColor(-64, 0, 96, 64, 0, 96, WIRE3DCGB_COLOR_HIGH);
        Wire3DCGB_DrawWhiteBorderFast();
        Wire3DCGB_EndFrameFast();
    }
}
```

## Surface And Color

- The wire surface is 128x96 pixels, mapped to a 16x12 BG tile region near the top of the screen.
- The 3072-byte 2bpp stage occupies WRAM bank 2 at `0xD300-0xDEFF`. Do not place application data in that bank/range.
- `Wire3DCGB_EndFrameFast()` transfers the completed stage to the inactive CGB VRAM tile bank with HBlank DMA, then changes the 16x12 tile-bank attributes during VBlank.
- By default the fast frame path updates tile-bank attributes, not LCDC. The optional atomic-map presenter below changes only LCDC bit 3 during VBlank; it never turns the LCD off during gameplay.
- Initialization selects a cleared VRAM tile bank, so slow first-frame rendering stays blank instead of exposing internal helper tiles.
- Color 0 is background. Colors 1-3 are line colors.
- `Wire3DCGB_SetPaletteRGB15()` sets BG palette 0.
- `Wire3DCGB_SetLineColor()` changes the current line color.
- `Wire3DCGB_DrawPoint2D()` sends one pixel directly to the assembly plotter, avoiding zero-length line setup for stars, particles, and radar dots.
- `Wire3DCGB_DrawLine2DColor()`, `Wire3DCGB_DrawLine3DColor()`, and `Wire3DCGB_DrawModelScaledColor()` draw a single item with a temporary color.
- Sparse rendering alternates VRAM banks. The upload covers current dirty tiles and the destination bank's N-2 dirty range, including erased objects. The stage is fully cleared before drawing each frame.
- `Wire3DCGB_EndFrameSparse()` includes an initial VBlank wait. `Wire3DCGB_EndFrameSparseNow()` omits that initial wait, but still waits for transfer completion and a safe presentation. It is synchronous, not an asynchronous submission API.
- Sparse GDMA is limited to 96 blocks starting on LY 144/145. Other uploads use HBlank DMA, armed from mode 2 with a short interrupt guard. Do not change source WRAM or destination VRAM banks while an upload is active.

## Atomic Maps And Clipping

For a 128x96 cockpit with a tile HUD, call `Wire3DCGB_EnableAtomicMaps()` once after initialization and initial direct BG/attribute setup. This reserves both BG maps (`0x9800` and `0x9C00`) and copies the HUD into both. Each map permanently selects a different VRAM tile bank for the wire window. Presentation then needs one LCDC map-select write during VBlank, after the back-bank upload finishes. LCD enable remains set.

Use `Wire3DCGB_PutBgTile()` for subsequent HUD tile changes; the queue mirrors writes into both maps. Direct runtime tile/attribute writes must update both maps explicitly. Do not use the second map independently for a window layer. The option is ignored in 160x144 mode and reset by initialization.

Call `Wire3DCGB_InvalidateFrameHistory()` on a scene reset or after bypassing the dirty-tracked drawing APIs. Both VRAM banks will be refreshed on their next presentation. In sparse mode, low-level `DrawLine2DColor` callers still need to mark their dirty rectangle; the clipped APIs below mark it automatically. A static border must be initialized in both banks before it is left out of dirty tracking.

- `Wire3DCGB_DrawLineClipped2D(x0,y0,x1,y1,color)` accepts signed endpoints in `[-2047,2047]`. It clips before converting to the rasterizer's byte coordinates. Values outside this range reject the line. Lines wholly inside the window bypass intersection arithmetic. Clipping is integer C; line rasterization is inline ASM.
- `Wire3DCGB_DrawEdgeListClipped2D(edges,edge_count,vx,vy,vertex_count,cx,cy,color)` accepts signed projected vertex offsets. Centers and offsets must each be in `[-2047,2047]`, and visible line endpoints must meet the line API limit. Invalid counts, indices, or null pointers reject the batch. Fully visible batches use the shared ASM edge loop; partial batches use signed clipping. It reuses internal projected-vertex scratch.
- `Wire3DCGB_ProjectAxis48(value,z)` returns signed `value * 48 / z`, truncated toward zero, using ASM normalization, multiplication and division. Depth is clamped to at least 4. For depth above 255, value and depth are repeatedly halved; normalized magnitude is limited to 160. It returns an offset, not a clipped screen position or a complete camera transform.
- `Wire3DCGB_EraseSpan2D(y,x0,x1)` clears a clipped horizontal 128x96 span using ASM and marks its dirty tiles. It is also available in the minimal runtime.

All drawing/projection entry points use shared scratch storage: call them from the main loop, not from an ISR or reentrantly. End-frame routines require an enabled LCD and normal main-loop interrupt operation. The 160x144 path still has its existing dynamic tile-pool limit; these changes do not remove that limit.

Executable checks and limitations for the 2026-09-07 changes are recorded in `../wire3d_shooting/verification/render_integrity_20260907/RESULTS.md`.

## Models

`Wire3DCGB_Model` stores vertices, edges, faces, and edge-to-face ownership. Set `WIRE3DCGB_MODEL_HIDDEN_LINES` to skip back-side edges from face orientation.

Limits:

- Vertices: `WIRE3DCGB_MODEL_VERTEX_LIMIT` 24
- Faces: `WIRE3DCGB_MODEL_FACE_LIMIT` 16
- Scene objects: `WIRE3DCGB_SCENE_OBJECT_LIMIT` 8

## Speed Notes

- `Wire3DCGB_Init()` switches to CGB double speed through KEY1/STOP.
- `Wire3DCGB_BeginFrame()` / `Wire3DCGB_EndFrame()` and their `Fast` variants use the same tear-free double-buffered presenter verified by KOKURA and SARAKURA.
- `Wire3DCGB_BeginFrameFast()` clears the stage and occlusion mask through inline assembly.
- `Wire3DCGB_EndFrameFast()` performs two HBlank DMA transfers totaling 3072 bytes and presents only after both complete.
- With atomic maps, use `Wire3DCGB_EndFrameSparseNow()` before a non-waiting OAM flush. It presents at VBlank; the caller remains responsible for fitting OAM/palette updates into the remaining safe period.
- `Wire3DCGB_DrawWhiteBorderFast()` draws the 128x96 four-sided border with palette color 2.
- The normal `Wire3DCGB_EndFrame()` also flushes queued BG tile writes when needed. `Wire3DCGB_EndFrameFast()` omits that check for the lowest call overhead.
- The 2bpp stage stores low and high bit planes separately in WRAM. The plot ASM updates the selected pixel's exact 2-bit color.
- `examples/wire3d_cgb_color_demo.c` demonstrates blue, white, and orange wire lines from the same BG palette.

## Hidden-Line Demo

Build the interactive hidden-line check with:

```powershell
.\examples\wire3d_cgb_hiddenline_demo_build.ps1
```

Controls:

- `START`: increase the visible object count from one to three; pressing it at three returns to one.
- `B`: switch the selected object.
- D-pad: move the selected object on X/Y.
- `A` + Up/Down: move the selected object on Z.
- `A` + Left/Right: rotate around Z in 22.5-degree steps.
- `SELECT` + D-pad: rotate around X/Y in 22.5-degree steps.

Hidden-line and inter-object occlusion remain enabled. The selected body is orange, overlapping bodies separate and bounce, and the white four-sided border stays fixed while objects move and rotate.

# wire3d_dmg: monochrome 3D for Game Boy

`wire3d_dmg.c` is a monochrome wireframe renderer for Game Boy. It projects 3D points and draws wireframe edges into a monochrome tile surface. `wire3d_cgb` provides color rendering.

## Choose a viewport before building

| Setting | 128 × 96 | 128 × 120 (default) |
| --- | --- | --- |
| Source to compile | `lib/wire3d_dmg_96.c` | `lib/wire3d_dmg.c` |
| Before including the header | `#define WIRE3D_DMG_HEIGHT 96` | `#define WIRE3D_DMG_HEIGHT 120` |
| `Wire3DDMG_BeginFrame()` | Clears the pixel stage and occlusion state | Resets occlusion state; uploaded pixels are cleared during transfer |
| Profile features | HUD and optional yaw-indexed edge masks | Optional dirty-tile and auxiliary transfer |
| Model limits | 24 vertices, 16 edges, 16 faces | 24 vertices, 16 faces; the model edge count is not capped at 16 |

Compile exactly one renderer entry. Use the same height in all files that include `wire3d_dmg.h`: model layouts and some angle parameter types depend on this setting. Selection happens at compile time and adds no per-frame profile branch.

## Build and run your first scene

Open PowerShell in the `kitaqgb` repository root. The compiler executable must be present there.

```powershell
# A rotating cube, 128 x 96.
.\kitaqgb.exe lib/wire3d_dmg_96.c examples/wire3d_dmg_96.c -I lib -o examples/wire3d_dmg_96.gb --profile=dev --stack-bank=fixed --rst-disable --no-disasm

# A centered cross, 128 x 120, with dirty-tile transfer.
.\kitaqgb.exe lib/wire3d_dmg.c examples/wire3d_dmg_120.c -I lib -o examples/wire3d_dmg_120.gb --profile=dev --stack-bank=fixed --rst-disable --no-disasm

# With the kokura repository beside kitaqgb:
..\kokura\kokura-cli.exe examples/wire3d_dmg_120.gb --run-frames 120 --png examples/wire3d_dmg_120.png
```

The first source argument selects the renderer profile; the second supplies the game. `-I lib` locates its header, and `-o` names the ROM. `--stack-bank=fixed` keeps stack storage out of the renderer's banked WRAM stage. The other options select the tested development build and disable RST optimization and disassembly output. The examples contain their own geometry and need no external art.

## Drawing a frame

```c
#define WIRE3D_DMG_HEIGHT 120
#include "wire3d_dmg.h"

void main()
{
    Wire3DDMG_Init();
    Wire3DDMG_SetDirtyTransfer(1); // Available in the 120-line profile.
    while (1) {
        Wire3DDMG_BeginFrame();
        Wire3DDMG_DrawLine3D(-24, 0, 100, 24, 0, 100);
        Wire3DDMG_DrawLine3D(0, -24, 100, 0, 24, 100);
        Wire3DDMG_EndFrame();
    }
}
```

Initialize once, draw the complete intended frame, and submit it with `EndFrame`. Uploading consumes the stage in both profiles, so redraw the scene each frame. In the 120-line profile, `BeginFrame` does not discard pixels that have not been uploaded. Dirty transfer also visits the preceding frame's dirty tiles to erase old pixels. Auxiliary transfer is disabled by default and shares part of the main stage.

`Wire3DDMG_DrawModel` accepts a `Wire3DDMG_Model` whose arrays remain readable in the active ROM bank. Initialize every field; the 96-line example demonstrates the complete model setup. Angles use 16 steps per revolution. `Wire3DDMG_DrawModelScaled` uses a Q8 scale, where 256 means the original size. Declarations and detailed contracts are in `wire3d_dmg.h`.

## Memory and visibility limits

Reserve WRAM `0xD000..0xDFFF` and retain its bank mapping while rendering. Initialization takes over the LCD tile data and background map; coordinate other display code accordingly. Renderer state is global and non-reentrant. Edges crossing the accepted depth range are omitted rather than clipped. Scene occlusion approximates visibility using face bounding boxes and five samples per line; it is not a depth buffer.

`EndFrame` waits for VBlank and then checks STAT during upload. A full transfer can continue beyond VBlank and does not promise 60 frames per second. The paired transfer loops use more ROM to reduce loop and STAT-check overhead, without allocating another stage buffer.

## Alternative entry points

`wire3d.c/h` select the 96-line profile with `Wire3D_*` functions; `dmg3d.c/h` select the 120-line profile with `DMG3D_*` functions. Match the source, header and function prefix in your program. Compile exactly one entry point. The `wire3d_dmg` interface provides both profiles through the `Wire3DDMG_*` prefix.

## Verification, September 13, 2026

Both presets and both compatibility entries were built and run in KOKURA. Transfer tests checked destination bytes, consumed source bytes, untouched bitplanes, and buffer boundaries; cube and cross displays were inspected.

At the tested VBlank entry, the measured transfer costs were 207,780 CPU cycles for the 120-line main stage, 34,928 for its auxiliary stage, and 164,164 for the 96-line stage. These are emulator measurements of transfer routines, not a whole-game frame-rate claim or a physical-hardware test.

[日本語](wire3d_dmg_guide_ja.md)

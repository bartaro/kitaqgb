# Wire3D clocked examples

MIT. The box geometry, eight bar tiles and four-note loop are independently authored. No commercial ROM, game code, map, screenshot, music or saved state is an input.

Choose one of `dmg_88`, `dmg_96`, `dmg_120`, `cgb_88`, `cgb_96`. The default library profiles remain DMG 120 and CGB 96. The examples draw 48 completed frames; two frames in every eight are blank to verify erasure. The BGM and gauge continue on VBlank even while rendering waits. Simulation advances by elapsed VBlank ticks, not by rendered-frame count. The byte counter requires polling within 255 VBlanks; use a safely read wider counter for longer stalls. Never call the shared-scratch renderer from an ISR.

From the kitaqgb repository root, with the native Rust compiler built:

```powershell
$profile = "cgb_88"
New-Item -ItemType Directory -Force out | Out-Null
.	argetelease\kitaqgb.exe compile examples/wire3d_clocked/hardware.c examples/wire3d_clocked/renderer_$profile.c examples/wire3d_clocked/audio_glue.c lib/wire3d_audio_vblank.c examples/wire3d_clocked/clocked_$profile.c -I lib -o out/wire3d.gb -O1 --stack-bank=fixed --stack-reserve=192 --rst-disable --no-disasm --romsize=256k --cart=mbc1
```

The legacy C# CLI uses the same arguments without `compile`. Compile exactly one renderer entry and one audio entry. `audio_glue.c` is BGM-only glue for this fixture; a full game can supply its own audio/SFX implementation. Do not also compile `audio_vblank.c`. `wire3d_audio_vblank.c` reserves 80 bytes in fixed WRAM and ROM bank 5 for controls. `AUDIO_VBLANK_QUEUE_WRAM0` keeps header declarations consistent. CGB renderer entries select `WIRE3DCGB_STAGE_BANK2_ALLOCATION`: the stage occupies bank 2 D300..DEFF (3072 reserved bytes in either height), leaving bank 1 for simulation. DMG reserves D000..DFFF for staging. Keep these regions and fixed ROM banks free.

`Wire3DDMG_EndFrameNow()` skips the initial fresh-VBlank wait but flushes queued BG writes and preserves the normal dirty/auxiliary transfer lifecycle. CGB `EndFrameSparseNow()` also retains synchronization and presentation waits. Both calls are synchronous; Now does not mean wait-free. DMG transfer is not an atomic presentation. CGB ordinary `DrawLine2D` requires explicit sparse dirty marking; the fixture uses `DrawLineClipped2D`.

CGB startup refreshes both destination banks over the first two sparse frames. Moving objects must also erase their earlier destination-bank contents. Full uploads and sparse histories are verified separately.

To run the source-only projection and runtime checks with a neighboring KOKURA source checkout (Python bridge and built C API DLL required; Pillow is optional for PNG output):

```powershell
python tools/wire3d_regression.py --compiler target/release/kitaqgb.exe --compiler-kind rust --kokura-dll ../kokura/target/release/kokura_capi.dll --bridge-dir ../kokura/python --output out/wire3d-check
python tools/wire3d_metrics.py out/wire3d-check/clocked_cgb_88_sparse/samples.json --deadline-frames 4
```

Six example-only markers separate clear, projection, rasterization, upload/presentation, and external pacing. Record completion after phase 4. PPU time is `frames*70224 + frame_phase_ppu_cycles`; 4194304 dots are one second on both DMG and CGB. Raw CGB CPU cycles use a different rate in double speed. Upload/presentation includes internal STAT, DMA and VBlank waits; the external pacing phase is separate. Mean, median, worst interval and strict deadline misses accompany completed-upload fps. Marker cost is included. Release library code contains no timing markers.

Emulator checks do not establish hardware performance. The four-PPU-frame pacing target is exceeded in some intervals. Smaller upload byte counts do not predict a proportional fps gain.

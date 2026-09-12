# KITAQGB CGB danmaku layer

`danmaku.h` / `danmaku.c` implement a bounded 96-bullet pool and a background
tile compositor for a 160 × 128 pixel playfield. They use no heap or OAM entries.
The bottom 16 screen pixels are available for a window HUD.

Positions are signed Q12.4 (`pixel * 16`), velocities signed Q4.4. The packed
bullet record is exactly eight bytes: `x16, y16, vx8, vy8, active8, grazed8`.
`danmaku_spawn(x,y,vx,vy)` returns 1 on success, 0 on rejection. A full pool
increments `dm_rejected` and never overwrites a live bullet. `dm_peak` records
the observed maximum. `danmaku_clear()` removes live bullets while retaining
session counters; `danmaku_reset()` also resets counters.

`danmaku_fan(x,y,direction,step,count,speed)` uses 32 angular positions: 0 right,
8 down, 16 left, 24 up. Speed is in sixteenths of a pixel and is capped at 64
(4 pixels per update). Count is capped at 96. Fan clipping at the screen edges
is intentional. Off-screen positions are culled before drawing or collision.

Every frame:

1. Call `danmaku_clear_map()` and populate decoration using tile IDs 16–255.
2. Set `dm_player_x`, `dm_player_y`, `dm_invulnerable`.
3. Call `danmaku_step()` to integrate, collide and compose bullets.
4. Consume `dm_hit` and `dm_graze`. Graze is emitted once per bullet.
5. At the **beginning of VBlank**, call `danmaku_present_now()`.

The 576-byte aligned map uses a 32-byte row stride. The last two rows can hold
scene text. The presenter uses 36-block CGB GDMA into VRAM bank 0, BG map 9800.
It restores VBK to 0 and does not wait for VBlank itself. The caller must keep
the source WRAM bank stable and schedule the transfer with enough blanking
time. This API does not support DMG or a simultaneously scrolling BG map.

Reserve BG tile IDs 0–15 for four quadrant masks. Bit 0 is top-left, bit 1
top-right, bit 2 bottom-left, bit 3 bottom-right. Each occupied 4 × 4 quadrant
has a 3 × 3 cross-shaped bullet centered at its local (2,2). The compositor
ORs multiple quadrants, so overlapping bullets never erase each other.
The visible positions are quantized to 4 pixels; collision uses those same
displayed centers, with ±3 pixel hit and ±8 pixel graze tests. The fixed-point
integrator retains subpixel velocity between visible steps.

No fast-projectile swept test is supplied. `danmaku_fan`'s maximum 4 px/update
matches this small-hitbox use case; callers spawning faster custom velocities
or moving targets by large jumps should substep their simulation.

The optimized assembly pass has the same public record layout as the C
reference in `ressen_gbc/tools/danmaku_reference.c`. The ROM self-test at
`ressen_gbc/tools/danmaku_selftest.c` checks signed movement, all four quadrant
masks, clipping on all edges, immunity, one-time graze, full-pool rejection,
cardinal fans and reset behavior (16 checks).

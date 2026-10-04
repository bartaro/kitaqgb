#pragma once

// Fixed-capacity, allocation-free bullet field for a 160 x 128 play area.
// Positions/velocities are signed Q12.4 / Q4.4; +Y points down.
// Tile ids 0..15 encode the four 4x4 quadrants of an 8x8 tile.
// Reserve those tiles in VRAM, with a 3x3 bullet centered at (2,2)
// in every set quadrant. Other tile ids in the map are decoration.
#define DANMAKU_MAX 96
#define DANMAKU_MAP_BYTES 576
typedef __packed struct {
    s16 x;
    s16 y;
    s8 vx;
    s8 vy;
    u8 active;
    u8 grazed;
} DanmakuBullet;

extern DanmakuBullet dm_bullets[DANMAKU_MAX];
extern __wram __aligned(16) u8 dm_map[DANMAKU_MAP_BYTES];
extern u8 dm_count;
extern u8 dm_peak;
extern u8 dm_hit;
extern u8 dm_graze;
extern u8 dm_player_x;
extern u8 dm_player_y;
extern u8 dm_invulnerable;
extern u16 dm_spawned;
extern u16 dm_rejected;

void danmaku_reset();
void danmaku_clear();
u8 danmaku_spawn(u8 x, u8 y, s8 vx, s8 vy);
void danmaku_fan(u8 x, u8 y, u8 direction, u8 step, u8 count, u8 speed);
// 32 directions, 0=right, 8=down, 16=left, 24=up. speed in 1/16 px.
void danmaku_clear_map();
// Caller fills decoration after clear_map, then calls step to overlay bullets.
// hit and graze are events for this step. Collision uses rendered centers.
void danmaku_step();
// Requires CGB and LCD off or the beginning of VBlank. VBK restored to 0.
// Transfers 18 rows (32-byte stride) to BG map 9800 with 36-block GDMA.
void danmaku_present_now();

/* Copyright (c) 2026 DAISUKE OBA. SPDX-License-Identifier: MIT.
   Game-specific collision operations. Functions below explain their state changes and
   hardware-facing responsibilities. */

/* Reuse eight launch directions to scatter all joints without random/trig work. */
/* Select one of eight signed Q4 horizontal launch speeds for a mine-hit fragment. */
static s16 SND_MissScatterVX(u8 index)
{
    index = (u8)(index & 7);
    if (index == 0) return (s16)(0 - 12);
    if (index == 1) return 0;
    if (index == 2) return 12;
    if (index == 3) return (s16)(0 - 14);
    if (index == 4) return 14;
    if (index == 5) return (s16)(0 - 10);
    if (index == 6) return 0;
    return 10;
}

/* Select the matching signed Q4 vertical launch speed for a mine-hit fragment. */
static s16 SND_MissScatterVY(u8 index)
{
    index = (u8)(index & 7);
    if (index == 0) return (s16)(0 - 10);
    if (index == 1) return (s16)(0 - 14);
    if (index == 2) return (s16)(0 - 10);
    if (index == 3) return 0;
    if (index == 4) return 0;
    if (index == 5) return 12;
    if (index == 6) return 14;
    return 12;
}

/* Snapshot the impact pose; the self-bite fade keeps these positions stationary. */
/* Snapshot each joint's impact position in Q4 units and assign scatter velocities; self-bites
   retain the snapshot without integrating it. */
static void SND_InitMissPieces()
{
    u8 i;

    i = 0;
    while (i < SND_Length) {
        SND_MissXQ4[(__safe_index u8)i] = (s16)((s16)SND_TraceX[(__safe_index u8)i] << 4);
        SND_MissYQ4[(__safe_index u8)i] = (s16)((s16)SND_TraceY[(__safe_index u8)i] << 4);
        SND_MissVXQ4[(__safe_index u8)i] = SND_MissScatterVX(i);
        SND_MissVYQ4[(__safe_index u8)i] = SND_MissScatterVY(i);
        i = (u8)(i + 1);
        SND_AudioServiceWork(i);
    }
}

/* Integrate each fragment's Q4 position and add downward acceleration after the initial
   scatter interval. */
static void SND_UpdateMissPieces()
{
    u8 i;

    i = 0;
    while (i < SND_Length) {
        SND_MissXQ4[(__safe_index u8)i] =
            (s16)(SND_MissXQ4[(__safe_index u8)i] + SND_MissVXQ4[(__safe_index u8)i]);
        SND_MissYQ4[(__safe_index u8)i] =
            (s16)(SND_MissYQ4[(__safe_index u8)i] + SND_MissVYQ4[(__safe_index u8)i]);
        if (SND_MissTimer > (u8)(SND_MISS_FLASH_FRAMES + 8)) {
            SND_MissVYQ4[(__safe_index u8)i] = (s16)(SND_MissVYQ4[(__safe_index u8)i] + 1);
        }
        i = (u8)(i + 1);
        SND_AudioServiceWork(i);
    }
}

/* Shared miss entry for mines and self-bites. Demo misses must neither affect
   rankings nor play SFX, but still run the visual animation. */
/* Enter the shared miss state: stage a player score, stop BGM, capture the pose and select the
   appropriate collision effect. */
static void SND_WallMiss()
{
    if (SND_DemoActive == 0) SND_RecordScore();
    SND_AudioStopBgm();
    SND_InitMissPieces();
    SND_MissTimer = 0;
    if (SND_CollisionKind == SND_COLLISION_MINE) {
        if (SND_DemoActive == 0) SND_AudioPlayMineExplosion();
        SND_DrawMineExplosionFrame();
    } else {
        if (SND_DemoActive == 0) SND_AudioPlayCrash();
    }
    SND_State = SND_STATE_MISS;
}

/* Test every joint against the two mine rectangles, retaining the first hit
   joint and mine index for the flash and aftermath rendering. */
/* Test every current segment center against the two inclusive mine rectangles and retain the
   first impact for animation. */
static u8 SND_CheckBlockMiss()
{
    u8 seg;
    u8 px;
    u8 py;

    seg = 0;
    while (seg < SND_Length) {
        py = SND_TraceY[(__safe_index u8)seg];
        if (py >= SND_BLOCK_MIN_Y && py <= SND_BLOCK_MAX_Y) {
            px = SND_TraceX[(__safe_index u8)seg];
            if (px >= SND_BLOCK_LEFT_MIN_X && px <= SND_BLOCK_LEFT_MAX_X) {
                SND_CollisionSegment = seg;
                SND_CollisionSegment2 = SND_SEGMENT_NONE;
                SND_CollisionKind = SND_COLLISION_MINE;
                SND_HitMineIndex = 0;
                return 1;
            }
            if (px >= SND_BLOCK_RIGHT_MIN_X && px <= SND_BLOCK_RIGHT_MAX_X) {
                SND_CollisionSegment = seg;
                SND_CollisionSegment2 = SND_SEGMENT_NONE;
                SND_CollisionKind = SND_COLLISION_MINE;
                SND_HitMineIndex = 1;
                return 1;
            }
        }
        seg = (u8)(seg + 1);
        SND_AudioServiceWork(seg);
    }
    return 0;
}

/* Return the unsigned absolute difference without wraparound adjustment. */
static u8 SND_AbsDiffU8(u8 a, u8 b)
{
    if (a > b) return (u8)(a - b);
    return (u8)(b - a);
}

/* A strict per-axis proximity test on the torus, not a circular-distance test. */
/* Test strict per-axis proximity using the shortest distance across the 160-by-144 wrapping
   field; this is a square, not a circular test. */
static u8 SND_WrappedHit(u8 ax, u8 ay, u8 bx, u8 by, u8 radius)
{
    u8 dx;
    u8 dy;

    dx = SND_AbsDiffU8(ax, bx);
    if (dx > (u8)(SND_SCREEN_W >> 1)) dx = (u8)(SND_SCREEN_W - dx);
    if (dx >= radius) return 0;

    dy = SND_AbsDiffU8(ay, by);
    if (dy > (u8)(SND_SCREEN_H >> 1)) dy = (u8)(SND_SCREEN_H - dy);
    if (dy < radius) {
        return 1;
    }
    return 0;
}

/* Compare the head with joints from index 2 onward and record both contact indices for the
   stationary fade animation. */
static u8 SND_CheckSelfMiss()
{
    u8 i;
    u8 px;
    u8 py;

    if (SND_Length <= SND_SELF_HIT_SKIP) return 0;

    /* Skip the head and first joint; their overlap is part of normal movement. */
    i = SND_SELF_HIT_SKIP;
    while (i < SND_Length) {
        px = SND_TraceX[(__safe_index u8)i];
        py = SND_TraceY[(__safe_index u8)i];
        if (SND_WrappedHit(SND_HeadX, SND_HeadY, px, py, SND_SELF_HIT_RADIUS) != 0) {
            /* Both contact sides flash before the stationary full-body fade. */
            SND_CollisionSegment = 0;
            SND_CollisionSegment2 = i;
            SND_CollisionKind = SND_COLLISION_SELF;
            SND_HitMineIndex = SND_MINE_NONE;
            return 1;
        }
        i = (u8)(i + 1);
        SND_AudioServiceWork(i);
    }
    return 0;
}

/* Consume a fruit within 7 pixels on both axes, grow the chain, increment the byte score,
   refresh speed/HUD and choose the next fruit. */
static void SND_CheckApple()
{
    u8 i;
    u8 ax;
    u8 ay;

    i = 0;
    while (i < SND_APPLE_COUNT) {
        if (SND_AppleActive[(__safe_index u8)i] != 0) {
            ax = SND_AppleX[(__safe_index u8)i];
            ay = SND_AppleY[(__safe_index u8)i];
            if (SND_AbsDiffU8(SND_HeadX, ax) < 7 &&
                SND_AbsDiffU8(SND_HeadY, ay) < 7) {
                SND_AppleActive[(__safe_index u8)i] = 0;
                SND_DrawAppleBg(i, 0);
                SND_AppleIndex = i;
                SND_ApplePosX = ax;
                SND_ApplePosY = ay;
                SND_Grow();
                if (SND_DemoActive == 0) SND_AudioPlayEat();
                SND_ApplesEaten = (u8)(SND_ApplesEaten + 1);
                SND_ResetSpeed();
                SND_World.max_speed = SND_CurrentMaxVelQ4;
                SND_DrawHud();
                SND_SpawnNextApple();
                return;
            }
        }
        i = (u8)(i + 1);
        SND_AudioServiceWork(i);
    }
}

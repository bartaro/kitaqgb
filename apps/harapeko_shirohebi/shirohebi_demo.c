/* Copyright (c) 2026 DAISUKE OBA. SPDX-License-Identifier: MIT.
   Game-specific demo operations. Functions below explain their state changes and hardware-
   facing responsibilities. */

/* A mid-turn pose, with the heading change already travelling down the body. */
static __prg_rom u8 SND_DemoPoseX[16] = {
    112, 108, 105, 102, 99, 96, 93, 89, 85, 81, 77, 73, 69, 65, 61, 57
};
static __prg_rom u8 SND_DemoPoseY[16] = {
    112, 110, 107, 104, 101, 98, 95, 94, 95, 97, 99, 101, 103, 105, 107, 110
};
static __prg_rom u8 SND_DemoPoseDir[16] = {
    1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1
};
static __prg_rom u8 SND_DemoWaveDir[8] = { 0, 1, 2, 1, 0, 15, 14, 15 };

/* Quantize the shortest wrapped route to the current fruit into eight targets. */
/* Quantize the shortest wrapped direction to the active fruit into eight target headings. */
static u8 SND_DemoTargetSector()
{
    s16 dx;
    s16 dy;
    u8 ax;
    u8 ay;

    dx = SND_ShortDelta(SND_ApplePosX, SND_HeadX, SND_SCREEN_W);
    dy = SND_ShortDelta(SND_ApplePosY, SND_HeadY, SND_SCREEN_H);
    ax = SND_AbsS16ToU8(dx);
    ay = SND_AbsS16ToU8(dy);
    if (ay <= (u8)(ax >> 1)) {
        if (dx < 0) return SND_DIR_LEFT;
        return SND_DIR_RIGHT;
    }
    if (ax <= (u8)(ay >> 1)) {
        if (dy < 0) return SND_DIR_UP;
        return SND_DIR_DOWN;
    }
    if (dx < 0) {
        if (dy < 0) return (u8)10;
        return (u8)6;
    }
    if (dy < 0) return (u8)14;
    return (u8)2;
}

/* Convert a chase target or the long-snake wave table into ordinary relative steering keys,
   preserving normal turn-repeat behavior. */
static u8 SND_DemoControlKeys()
{
    u8 desired;
    u8 delta;

    if (SND_DemoScene != 0) {
        /* Step with the physics, so missed display frames cannot skew the wave. */
        desired = SND_DemoWaveDir[(__safe_index u8)(SND_DemoWeaveStep >> 3)];
        SND_DemoWeaveStep = (u8)((SND_DemoWeaveStep + 1) & 63);
    } else {
        desired = SND_DemoTargetSector();
    }
    /* Feed normal relative steering keys, preserving the game's turn repeat. */
    delta = desired;
    if (delta < SND_Facing) delta = (u8)(delta + SND_DIR_STEPS);
    delta = (u8)(delta - SND_Facing);

    if (delta == 0) return 0;
    if (delta <= (u8)(SND_DIR_STEPS >> 1)) return PAD_KEY_RIGHT;
    return PAD_KEY_LEFT;
}

/* Reset elapsed demo time and capture the current modulo-256 interrupt counter. */
static void SND_ResetDemoClock()
{
    SND_DemoSeconds = 0;
    SND_DemoSubTicks = 0;
    SND_DemoVBlankPrevious = SND_DemoVBlankCounter;
}

/* Consume the modulo-256 ISR counter, including VBlanks skipped by a slow
   gameplay tick. This keeps the demo timeout independent of joint workload. */
/* Consume elapsed VBlank ticks, including skipped main-loop frames, and report the
   30-by-60-tick demo limit. */
static u8 SND_DemoClockExpired()
{
    u8 now;
    u8 elapsed;

    now = SND_DemoVBlankCounter;
    if (now >= SND_DemoVBlankPrevious) {
        elapsed = (u8)(now - SND_DemoVBlankPrevious);
    } else {
        elapsed = (u8)((u8)(255 - SND_DemoVBlankPrevious) + now + 1);
    }
    SND_DemoVBlankPrevious = now;
    while (elapsed != 0) {
        elapsed = (u8)(elapsed - 1);
        SND_DemoSubTicks = (u8)(SND_DemoSubTicks + 1);
        if (SND_DemoSubTicks >= SND_DEMO_VBLANK_TICKS_PER_SECOND) {
            SND_DemoSubTicks = 0;
            SND_DemoSeconds = (u8)(SND_DemoSeconds + 1);
            if (SND_DemoSeconds >= SND_DEMO_PLAY_SECONDS) return 1;
        }
    }
    return 0;
}

/* Alternate fruit-chase and long-snake attract scenes; the latter installs a 16-segment pose
   directly into the shared chain arrays. */
static void SND_StartDemo()
{
    u8 i;

    SND_DisableInputShortcuts();

    /* Boot initializes Scene to 1, so the first idle demo is the fruit chase. */
    SND_DemoScene = (u8)(SND_DemoScene ^ 1);
    SND_DemoActive = 1;
    SND_ResetDemoClock();
    SND_TitleIdleFrames = 0;
    SND_StartRun();
    if (SND_DemoScene != 0) {
        /* Head plus 15 joints below the mines; no ready/countdown screen. */
        SND_Length = 16;
        /* Score the extra joints beyond the normal three-joint starting body. */
        SND_ApplesEaten = (u8)((SND_Length - 1) - SND_START_BODY_COUNT);
        SND_DrawHud();
        SND_HeadX = 112;
        SND_HeadY = 112;
        /* Use explicit Q4 constants: shifting an 8-bit literal can truncate. */
        SND_HeadBody.x = (s16)1792;
        SND_HeadBody.y = (s16)1792;
        SND_HeadBody.vx = SND_StartVelocity();
        SND_HeadBody.vy = (s16)2;
        SND_Facing = 1;
        SND_DemoWeaveStep = 25;
        i = 0;
        while (i < SND_Length) {
            SND_TraceX[(__safe_index u8)i] = SND_DemoPoseX[(__safe_index u8)i];
            SND_TraceY[(__safe_index u8)i] = SND_DemoPoseY[(__safe_index u8)i];
            SND_TraceDir[(__safe_index u8)i] = SND_DemoPoseDir[(__safe_index u8)i];
            i = (u8)(i + 1);
        }
        SND_DrawDemoLabel();
        SND_ResetDemoClock();
    }
}

/* Use the same release lock and overlay cleanup as the in-game shortcut. */
/* Leave the attract scene through normal title cleanup and its release-to-rearm input gate. */
static void SND_EndDemo()
{
    SND_ReturnToTitle();
}

/* Copyright (c) 2026 DAISUKE OBA. SPDX-License-Identifier: MIT.
   Game-specific snake operations. Functions below explain their state changes and hardware-
   facing responsibilities. */

/* One base-speed increase per five points, capped at the 50-point stage. */
/* Compute min(score / 5, 10) by repeated subtraction to avoid a general division helper. */
static u8 SND_SpeedStage()
{
    u8 score;
    u8 stage;

    score = SND_ApplesEaten;
    stage = 0;
    while (score >= 5 && stage < SND_SPEED_STAGE_MAX) {
        score = (u8)(score - 5);
        stage = (u8)(stage + 1);
    }
    return stage;
}

/* Return the score-dependent speed limit in Q4 units: 8 plus 2 units per stage. */
static s16 SND_TargetVelocity()
{
    return (s16)(SND_BASE_MAX_VEL_Q4 + (s16)(SND_SpeedStage() * SND_SPEED_STEP_Q4));
}

/* Compute the first UP-held target as base plus one quarter of base, using integer truncation. */
static s16 SND_BoostStartVelocity(s16 base)
{
    return (s16)(base + (base >> 2));
}

/* Return the per-update boost/release ramp increment, at least one Q4 unit. */
static s16 SND_BoostStep(s16 base)
{
    s16 step;

    step = (s16)(base >> 2);
    if (step < SND_BOOST_STEP_MIN_Q4) step = SND_BOOST_STEP_MIN_Q4;
    return step;
}

/* UP jumps to 125% on the first held tick, then ramps toward 200%.
   Releasing it eases back to the score-dependent base speed. */
/* Jump to 125 percent on a fresh UP hold, ramp toward twice base while held and ease back to
   base on release; update the world's speed limit. */
static void SND_UpdateSpeedForKeys(u8 keys)
{
    s16 base;
    s16 start;
    s16 maximum;
    s16 step;

    base = SND_TargetVelocity();
    start = SND_BoostStartVelocity(base);
    maximum = (s16)(base << 1);
    step = SND_BoostStep(base);
    if ((keys & PAD_KEY_UP) != 0) {
        if (SND_UpBoostHeld == 0 || SND_CurrentMaxVelQ4 < start) {
            SND_CurrentMaxVelQ4 = start;
        } else if (SND_CurrentMaxVelQ4 < maximum) {
            SND_CurrentMaxVelQ4 = (s16)(SND_CurrentMaxVelQ4 + step);
            if (SND_CurrentMaxVelQ4 > maximum) SND_CurrentMaxVelQ4 = maximum;
        }
        SND_UpBoostHeld = 1;
    } else {
        SND_UpBoostHeld = 0;
        if (SND_CurrentMaxVelQ4 > base) {
            SND_CurrentMaxVelQ4 = (s16)(SND_CurrentMaxVelQ4 - step);
            if (SND_CurrentMaxVelQ4 < base) SND_CurrentMaxVelQ4 = base;
        } else if (SND_CurrentMaxVelQ4 < base) {
            SND_CurrentMaxVelQ4 = base;
        }
    }
    SND_World.max_speed = SND_CurrentMaxVelQ4;
}

/* Set the current speed limit directly to the score-dependent base. */
static void SND_ResetSpeed()
{
    SND_CurrentMaxVelQ4 = SND_TargetVelocity();
}

/* Return the base speed used when initializing or recovering head motion. */
static s16 SND_StartVelocity()
{
    return SND_TargetVelocity();
}

/* Return the maximum per-axis velocity correction of three Q4 units per gameplay update. */
static s16 SND_AlignStep()
{
    return SND_ALIGN_STEP_Q4;
}

/* Clamp each signed head-velocity component to the current positive/negative speed limit. */
static void SND_ClampHeadVelocity()
{
    s16 min_vel;

    min_vel = (s16)(0 - SND_CurrentMaxVelQ4);
    if (SND_HeadBody.vx > SND_CurrentMaxVelQ4) SND_HeadBody.vx = SND_CurrentMaxVelQ4;
    if (SND_HeadBody.vx < min_vel) SND_HeadBody.vx = min_vel;
    if (SND_HeadBody.vy > SND_CurrentMaxVelQ4) SND_HeadBody.vy = SND_CurrentMaxVelQ4;
    if (SND_HeadBody.vy < min_vel) SND_HeadBody.vy = min_vel;
}

/* Create a single zero-gravity physics body for the head and initialize a four-segment chain
   sharing the rendering/collision arrays. */
static void SND_ResetSnake()
{
    SND_UpBoostHeld = 0;
    SND_Length = SND_START_LENGTH;
    SND_HeadBody.x = SND_START_X_Q4;
    SND_HeadBody.y = SND_START_Y_Q4;
    SND_HeadBody.vx = 0;
    SND_HeadBody.vy = (s16)(0 - SND_StartVelocity());
    SND_HeadBody.half_x = SND_HALF_SIZE_Q4;
    SND_HeadBody.half_y = SND_HALF_SIZE_Q4;
    SND_HeadBody.inv_mass_q8 = 256;
    SND_HeadBody.restitution_q8 = 0;
    SND_HeadBody.active = 1;
    kq2d_world_init(&SND_World, &SND_HeadBody, 1);
    SND_World.gravity_x = 0;
    SND_World.gravity_y = 0;
    /* Only the head is a physics body. Joint following and hit detection are
       handled explicitly, so the generic contact solver is not used. */
    SND_World.solver_iterations = 0;
    SND_World.max_speed = SND_CurrentMaxVelQ4;
    SND_HeadX = SND_START_X;
    SND_HeadY = SND_START_Y;
    SND_Facing = SND_DIR_UP;
    SND_TailOamPhase = 0;

    /* Share the existing pose arrays with collision/rendering; no copy or
       resampling is needed. Capacity includes the head plus 30 joints. */
    chain_body_init(&SND_ChainBody, SND_TraceX, SND_TraceY, SND_TraceDir,
                    SND_MAX_SEGMENTS, SND_SCREEN_W, SND_SCREEN_H);
    chain_body_reset(&SND_ChainBody, SND_Length, SND_HeadX, SND_HeadY, SND_Facing);
}

/* Reset score, pose, fruit, timers and effects; enter READY for a fresh run or PLAY for the
   installed long-snake demo pose. */
static void SND_StartRun()
{
    SND_AppleIndex = 0;
    SND_ApplesEaten = 0;
    SND_ResetSpeed();
    SND_ResetSnake();
    SND_MissTimer = 0;
    SND_PauseTimer = 0;
    SND_PauseShown = 0;
    SND_PauseDesired = 0;
    SND_PauseDirty = 0;
    SND_StartTimer = 0;
    SND_StartShown = 0;
    SND_StartDesired = 0;
    SND_StartDirty = 0;
    SND_TurnRepeatDir = SND_STEER_NONE;
    SND_TurnRepeatTimer = 0;
    SND_CollisionSegment = 0;
    SND_CollisionSegment2 = SND_SEGMENT_NONE;
    SND_CollisionKind = SND_COLLISION_NONE;
    SND_OBP1 = 0xE4;
    SND_HitMineIndex = SND_MINE_NONE;
    SND_GrowFlashSegment = SND_SEGMENT_NONE;
    SND_GrowFlashTimer = 0;
    SND_GameOverChoice = SND_GAMEOVER_YES;
    SND_PendingRank = SND_PENDING_NONE;
    SND_BuildBackground();
    SND_ResetApples();
    SND_DrawHud();
    /* The long-snake attract scene is a mid-run view, not a fresh-game intro. */
    if (SND_DemoActive != 0 && SND_DemoScene != 0) {
        SND_State = SND_STATE_PLAY;
    } else {
        SND_SetStartVisible(1);
        SND_State = SND_STATE_READY;
    }
    if (SND_DemoActive != 0) SND_AudioStopBgm();
    else SND_AudioStartGameJingle();
}

/* Consume the start press, leave demo mode and initialize a new player run. */
static void SND_StartGame()
{
    SND_LockSystemButtons();
    SND_DemoActive = 0;
    SND_DemoSeconds = 0;
    SND_DemoSubTicks = 0;
    SND_TitleIdleFrames = 0;
    SND_StartRun();
}

/* Append a copy of the tail within capacity, synchronize the segment count and highlight the
   former tail while following separates the new joint. */
static void SND_Grow()
{
    u8 prev;

    if (SND_Length >= SND_MAX_SEGMENTS) return;
    /* Duplicate the old tail to avoid a gap; following updates separate it.
       The old tail becomes the yellow joint immediately before the new tail. */
    prev = (u8)(SND_Length - 1);
    SND_ChainBody.count = SND_Length;
    if (chain_body_grow(&SND_ChainBody) == 0) return;
    SND_GrowFlashSegment = prev;
    SND_GrowFlashTimer = SND_GROW_FLASH_FRAMES;
    SND_Length = SND_ChainBody.count;
}

/* Subtract one of sixteen clockwise heading sectors with wraparound. */
static u8 SND_SectorLeft(u8 sector)
{
    if (sector == 0) return (u8)(SND_DIR_STEPS - 1);
    return (u8)(sector - 1);
}

/* Add one heading sector with wraparound at sixteen. */
static u8 SND_SectorRight(u8 sector)
{
    sector = (u8)(sector + 1);
    if (sector >= SND_DIR_STEPS) return 0;
    return sector;
}

/* Return a signed displacement's magnitude saturated to 255 for demo direction selection. */
static u8 SND_AbsS16ToU8(s16 v)
{
    if (v < 0) v = (s16)(0 - v);
    if (v > 255) return 255;
    return (u8)v;
}

/* Rotate the intended heading one sector counterclockwise; velocity aligns gradually
   afterward. */
static void SND_TurnLeft()
{
    SND_Facing = SND_SectorLeft(SND_Facing);
}

/* Rotate the intended heading one sector clockwise; velocity aligns gradually afterward. */
static void SND_TurnRight()
{
    SND_Facing = SND_SectorRight(SND_Facing);
}

/* Move one velocity component toward its target by at most three Q4 units without
   overshooting. */
static s16 SND_ApproachVelocity(s16 value, s16 target)
{
    s16 step;

    step = SND_AlignStep();
    if (value < target) {
        value = (s16)(value + step);
        if (value > target) value = target;
    } else if (value > target) {
        value = (s16)(value - step);
        if (value < target) value = target;
    }
    return value;
}

/* Sixteen clockwise sectors start at right. Quarter/diagonal components
   approximate headings without floating-point trigonometry on the GB CPU. */
/* Approximate the horizontal component of a sixteen-sector heading using full, three-quarter,
   quarter and zero speed values. */
static s16 SND_DirVelocityX(u8 sector)
{
    s16 max_vel;
    s16 quarter;
    s16 diagonal;

    max_vel = SND_CurrentMaxVelQ4;
    quarter = (s16)(max_vel >> 2);
    diagonal = (s16)(max_vel - quarter);

    if (sector == 0) return max_vel;
    if (sector == 1) return max_vel;
    if (sector == 2) return diagonal;
    if (sector == 3) return quarter;
    if (sector == 4) return 0;
    if (sector == 5) return (s16)(0 - quarter);
    if (sector == 6) return (s16)(0 - diagonal);
    if (sector == 7) return (s16)(0 - max_vel);
    if (sector == 8) return (s16)(0 - max_vel);
    if (sector == 9) return (s16)(0 - max_vel);
    if (sector == 10) return (s16)(0 - diagonal);
    if (sector == 11) return (s16)(0 - quarter);
    if (sector == 12) return 0;
    if (sector == 13) return quarter;
    if (sector == 14) return diagonal;
    return max_vel;
}

/* Return the corresponding vertical component with positive Y downward, avoiding floating-
   point trigonometry. */
static s16 SND_DirVelocityY(u8 sector)
{
    s16 max_vel;
    s16 quarter;
    s16 diagonal;

    max_vel = SND_CurrentMaxVelQ4;
    quarter = (s16)(max_vel >> 2);
    diagonal = (s16)(max_vel - quarter);

    if (sector == 0) return 0;
    if (sector == 1) return quarter;
    if (sector == 2) return diagonal;
    if (sector == 3) return max_vel;
    if (sector == 4) return max_vel;
    if (sector == 5) return max_vel;
    if (sector == 6) return diagonal;
    if (sector == 7) return quarter;
    if (sector == 8) return 0;
    if (sector == 9) return (s16)(0 - quarter);
    if (sector == 10) return (s16)(0 - diagonal);
    if (sector == 11) return (s16)(0 - max_vel);
    if (sector == 12) return (s16)(0 - max_vel);
    if (sector == 13) return (s16)(0 - max_vel);
    if (sector == 14) return (s16)(0 - diagonal);
    return (s16)(0 - quarter);
}

/* Build the intended velocity vector and ease the actual X/Y components toward it
   independently. */
static void SND_ApplyHeadingAccel()
{
    s16 target_vx;
    s16 target_vy;

    target_vx = SND_DirVelocityX(SND_Facing);
    target_vy = SND_DirVelocityY(SND_Facing);
    SND_HeadBody.vx = SND_ApproachVelocity(SND_HeadBody.vx, target_vx);
    SND_HeadBody.vy = SND_ApproachVelocity(SND_HeadBody.vy, target_vy);
}

/* Emit one relative turn immediately, then repeat after the initial delay.
   Opposing held keys cancel; releasing clears the repeat state. */
/* Emit an immediate relative turn, then use the first/repeat countdowns; simultaneous
   LEFT/RIGHT or release cancels repeating. */
static u8 SND_UpdateTurnRepeat(u8 keys)
{
    u8 dir;

    dir = SND_STEER_NONE;
    if ((keys & PAD_KEY_LEFT) != 0 && (keys & PAD_KEY_RIGHT) == 0) {
        dir = SND_STEER_LEFT;
    } else if ((keys & PAD_KEY_RIGHT) != 0 && (keys & PAD_KEY_LEFT) == 0) {
        dir = SND_STEER_RIGHT;
    }

    if (dir == SND_STEER_NONE) {
        SND_TurnRepeatDir = SND_STEER_NONE;
        SND_TurnRepeatTimer = 0;
        return SND_STEER_NONE;
    }
    if (dir != SND_TurnRepeatDir) {
        SND_TurnRepeatDir = dir;
        SND_TurnRepeatTimer = SND_REPEAT_FIRST;
        return dir;
    }
    if (SND_TurnRepeatTimer != 0) {
        SND_TurnRepeatTimer = (u8)(SND_TurnRepeatTimer - 1);
        return SND_STEER_NONE;
    }

    SND_TurnRepeatTimer = SND_REPEAT_NEXT;
    return dir;
}

/* Apply one relative turn, align velocity, recover a stopped head if necessary and clamp both
   velocity components. */
static void SND_ApplySteering(u8 keys)
{
    u8 steer;

    steer = SND_UpdateTurnRepeat(keys);
    if (steer == SND_STEER_LEFT) {
        SND_TurnLeft();
    } else if (steer == SND_STEER_RIGHT) {
        SND_TurnRight();
    }
    SND_ApplyHeadingAccel();
    if (SND_HeadBody.vx == 0 && SND_HeadBody.vy == 0) {
        SND_HeadBody.vx = SND_StartVelocity();
        SND_Facing = SND_DIR_RIGHT;
    }
    SND_ClampHeadVelocity();
}

/* Round Q4 centers once, so rendering and collision checks see the same head. */
/* Round Q4 head coordinates once using (value + 8) >> 4 so drawing and collision share
   identical pixel centers. */
static void SND_SyncHeadPixel()
{
    SND_HeadX = (u8)((SND_HeadBody.x + 8) >> SND_PHYS_SHIFT);
    SND_HeadY = (u8)((SND_HeadBody.y + 8) >> SND_PHYS_SHIFT);
}

/* Teleport the head's center across the configured 4..156 and 4..140 pixel boundaries when it
   leaves them. */
static void SND_WrapHead()
{
    if (SND_HeadBody.x < SND_WRAP_MIN_X_Q4) {
        SND_HeadBody.x = SND_WRAP_MAX_X_Q4;
    } else if (SND_HeadBody.x > SND_WRAP_MAX_X_Q4) {
        SND_HeadBody.x = SND_WRAP_MIN_X_Q4;
    }

    if (SND_HeadBody.y < SND_WRAP_MIN_Y_Q4) {
        SND_HeadBody.y = SND_WRAP_MAX_Y_Q4;
    } else if (SND_HeadBody.y > SND_WRAP_MAX_Y_Q4) {
        SND_HeadBody.y = SND_WRAP_MIN_Y_Q4;
    }
}

/* Choose the shortest signed displacement across the wraparound playfield. */
/* Choose the shortest signed displacement across a wrapping field, retaining the direct sign
   at exactly half its size. */
static s16 SND_ShortDelta(u8 cur, u8 prev, u8 size)
{
    s16 d;
    s16 half;

    d = (s16)((s16)cur - (s16)prev);
    half = (s16)(size >> 1);
    if (d > half) d = (s16)(d - (s16)size);
    else if (d < (s16)(0 - half)) d = (s16)(d + (s16)size);
    return d;
}

/* The library retains the previous heading at each joint, so a turn travels
   down the body one joint per update. Sync count for the 15-joint demo,
   which installs its custom pose directly into the shared arrays. */
/* Synchronize the active count and advance the library follower in the shared pose arrays;
   each joint passes its saved heading to the next joint. */
static void SND_UpdateArticulatedBody()
{
    SND_ChainBody.count = SND_Length;
    chain_body_step(&SND_ChainBody, SND_HeadX, SND_HeadY, SND_Facing);
}

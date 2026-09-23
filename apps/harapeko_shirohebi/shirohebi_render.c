/* Copyright (c) 2026 DAISUKE OBA. SPDX-License-Identifier: MIT.
   Game-specific render operations. Functions below explain their state changes and hardware-
   facing responsibilities. */

/* Convert an 8x8 sprite's pixel center to its logical top-left corner.
   SND_OamSet adds the hardware X/Y offsets separately. */
/* Convert an 8-by-8 sprite's center to its top-left coordinate; hardware offsets are added by
   SND_OamSet. */
static u8 SND_SpritePos(u8 center)
{
    return (u8)(center - 4);
}

/* Use the growth-highlight palette for the selected joint while its timer is active; use the
   snake palette otherwise. */
static u8 SND_SegmentAttr(u8 segment)
{
    if (SND_GrowFlashTimer != 0 && segment == SND_GrowFlashSegment) {
        return SND_OBJ_GROW_ATTR;
    }
    return KQ_CGB_ATTR_PAL(0);
}

/* Read one joint from the shared pose arrays and write its position, tile and palette to the
   requested shadow OAM slot. */
static void SND_RenderTraceSegment(u8 oam, u8 segment, u8 tile)
{
    u8 px;
    u8 py;

    px = SND_TraceX[(__safe_index u8)segment];
    py = SND_TraceY[(__safe_index u8)segment];
    SND_OamSet(oam, SND_SpritePos(px), SND_SpritePos(py), tile, SND_SegmentAttr(segment));
}

/* Sixteen steering sectors share eight face graphics; eye direction follows
   the heading even while inertia is still aligning the actual velocity. */
/* Map sixteen steering headings to eight directional head graphics; eye direction follows
   intent rather than the lagging velocity vector. */
static u8 SND_HeadTile()
{
    if (SND_Facing == 1 || SND_Facing == 2 || SND_Facing == 3) return SND_TILE_HEAD_DR;
    if (SND_Facing == SND_DIR_DOWN) return SND_TILE_HEAD_DOWN;
    if (SND_Facing == 5 || SND_Facing == 6 || SND_Facing == 7) return SND_TILE_HEAD_DL;
    if (SND_Facing == SND_DIR_LEFT) return SND_TILE_HEAD_LEFT;
    if (SND_Facing == 9 || SND_Facing == 10 || SND_Facing == 11) return SND_TILE_HEAD_UL;
    if (SND_Facing == SND_DIR_UP) return SND_TILE_HEAD_UP;
    if (SND_Facing == 13 || SND_Facing == 14 || SND_Facing == 15) return SND_TILE_HEAD_UR;
    return SND_TILE_HEAD;
}

/* Choose a head, body or tail tile from the segment's position in the chain. */
static u8 SND_SnakeSegmentTile(u8 segment, u8 last)
{
    if (segment == 0) return SND_HeadTile();
    if (segment == last) return SND_TILE_TAIL;
    return SND_TILE_BODY;
}

/* Keep the first six joints at stable OAM priority and rotate rear-joint order to share the
   ten-sprites-per-line limit over frames. */
static void SND_RenderMovingSnakeSprites()
{
    u8 i;
    u8 last;
    u8 fixed;
    u8 tail_count;
    u8 phase;
    u8 seg;
    u8 oam;

    if (SND_Length == 0) return;
    last = (u8)(SND_Length - 1);
    SND_RenderTraceSegment(SND_OAM_SNAKE_BASE, 0, SND_HeadTile());

    i = 1;
    if (SND_Length <= SND_OAM_SNAKE_FIXED_FRONT) {
        while (i < SND_Length) {
            SND_RenderTraceSegment((u8)(SND_OAM_SNAKE_BASE + i), i, SND_SnakeSegmentTile(i, last));
            i = (u8)(i + 1);
            SND_AudioServiceWork(i);
        }
        return;
    }

    /* Keep the head/front joints at stable, high-priority OAM indices. */
    fixed = SND_OAM_SNAKE_FIXED_FRONT;
    while (i < fixed) {
        SND_RenderTraceSegment((u8)(SND_OAM_SNAKE_BASE + i), i, SND_TILE_BODY);
        i = (u8)(i + 1);
        SND_AudioServiceWork(i);
    }

    tail_count = (u8)(SND_Length - fixed);
    phase = SND_TailOamPhase;
    while (phase >= tail_count) phase = (u8)(phase - tail_count);

    /* Write every rear joint, but rotate their OAM order. This shares the GB's
       ten-sprites-per-scanline limit over successive renders without deleting joints. */
    seg = (u8)(fixed + phase);
    oam = fixed;
    while (oam < SND_Length) {
        SND_RenderTraceSegment((u8)(SND_OAM_SNAKE_BASE + oam), seg, SND_SnakeSegmentTile(seg, last));
        oam = (u8)(oam + 1);
        seg = (u8)(seg + 1);
        if (seg >= SND_Length) seg = fixed;
        SND_AudioServiceWork(oam);
    }

    SND_TailOamPhase = (u8)(phase + SND_OAM_SNAKE_ROTATE_STEP);
    while (SND_TailOamPhase >= tail_count) {
        SND_TailOamPhase = (u8)(SND_TailOamPhase - tail_count);
    }
}

/* Round a Q4 fragment center to pixels, clip off-screen fragments and emit its shadow OAM
   entry. */
static void SND_OamSetCenterQ4(u8 oam, s16 xq4, s16 yq4, u8 tile, u8 attr)
{
    s16 x;
    s16 y;

    x = (s16)((xq4 + 8) >> 4);
    y = (s16)((yq4 + 8) >> 4);
    if (x < 4 || x > 156 || y < 4 || y > 140) return;
    SND_OamSet(oam, (u8)(x - 4), (u8)(y - 4), tile, attr);
}

/* Self-bites freeze the impact pose: flash both contacts, then fade all joints
   through light gray, dark gray and blue (darkest shade on DMG), without scatter. */
/* Render the frozen self-bite pose, flash both contacts and fade the full body through timed
   CGB palettes or DMG shades. */
static void SND_RenderSelfFade()
{
    u8 i;
    u8 last;
    u8 tile;
    u8 attr;

    if (SND_MissTimer >= SND_SELF_FADE_HIDE_FRAME) return;

    if (SND_MissTimer < SND_SELF_FADE_LIGHT_FRAME) {
        SND_OBP1 = 0xE4;
    } else if (SND_MissTimer < SND_SELF_FADE_DARK_FRAME) {
        SND_OBP1 = SND_DMG_FADE_LIGHT;
    } else if (SND_MissTimer < SND_SELF_FADE_BLUE_FRAME) {
        SND_OBP1 = SND_DMG_FADE_DARK;
    } else {
        SND_OBP1 = SND_DMG_FADE_BLUE;
    }

    last = (u8)(SND_Length - 1);
    i = 0;
    while (i < SND_Length) {
        tile = SND_SnakeSegmentTile(i, last);

        if (SND_MissTimer < SND_SELF_FADE_LIGHT_FRAME) {
            attr = KQ_CGB_ATTR_PAL(0);
            if (i == SND_CollisionSegment || i == SND_CollisionSegment2) {
                attr = SND_OBJ_HIT_ATTR;
            }
        } else if (SND_MissTimer < SND_SELF_FADE_DARK_FRAME) {
            attr = (u8)(KQ_CGB_ATTR_PAL(5) | SND_OBJ_DMG_PAL1);
        } else if (SND_MissTimer < SND_SELF_FADE_BLUE_FRAME) {
            attr = (u8)(KQ_CGB_ATTR_PAL(6) | SND_OBJ_DMG_PAL1);
        } else {
            attr = (u8)(KQ_CGB_ATTR_PAL(7) | SND_OBJ_DMG_PAL1);
        }

        SND_OamSetCenterQ4((u8)(SND_OAM_SNAKE_BASE + i),
                           SND_MissXQ4[(__safe_index u8)i],
                           SND_MissYQ4[(__safe_index u8)i],
                           tile,
                           attr);
        i = (u8)(i + 1);
        SND_AudioServiceWork(i);
    }
}

/* Choose stationary self-fade or mine scatter; stagger fragment bursts so groups do not
   disappear simultaneously. */
static void SND_RenderMissAnimation()
{
    u8 i;
    u8 last;
    u8 tile;
    u8 attr;
    u8 burst_start;

    if (SND_CollisionKind == SND_COLLISION_SELF) {
        SND_RenderSelfFade();
        return;
    }

    last = (u8)(SND_Length - 1);
    i = 0;
    while (i < SND_Length) {
        tile = SND_SnakeSegmentTile(i, last);
        attr = KQ_CGB_ATTR_PAL(0);

        if (SND_MissTimer < SND_MISS_FLASH_FRAMES) {
            if (i == SND_CollisionSegment || i == SND_CollisionSegment2) {
                attr = SND_OBJ_HIT_ATTR;
            }
        } else {
            /* Stagger four groups so the scattered joints do not vanish at once. */
            burst_start = (u8)(SND_MISS_BURST_FRAME + (u8)((i & 3) * 3));
            if (SND_MissTimer >= burst_start) {
                if (SND_MissTimer >= (u8)(burst_start + SND_MISS_BURST_LEN)) {
                    i = (u8)(i + 1);
                    continue;
                }
                tile = SND_TILE_BURST;
                if ((SND_MissTimer & 2) != 0) tile = SND_TILE_FLASH;
                attr = KQ_CGB_ATTR_PAL(1);
            }
        }

        SND_OamSetCenterQ4((u8)(SND_OAM_SNAKE_BASE + i),
                           SND_MissXQ4[(__safe_index u8)i],
                           SND_MissYQ4[(__safe_index u8)i],
                           tile,
                           attr);
        i = (u8)(i + 1);
        SND_AudioServiceWork(i);
    }
}

/* Prepare shadow OAM for play/pause/ready, miss animation or a sprite-free menu, always
   clearing unneeded slots. */
static void SND_RenderSprites()
{
    if (SND_State == SND_STATE_PLAY ||
        SND_State == SND_STATE_PAUSE ||
        SND_State == SND_STATE_READY) {
        SND_OamHideUnusedPlaySlots(SND_Length);
        SND_OamHideTextSlots();
        SND_RenderMovingSnakeSprites();
    } else if (SND_State == SND_STATE_MISS) {
        SND_OamHidePlaySlots();
        SND_OamHideTextSlots();
        if (SND_MissTimer < SND_MISS_CLEAR_FRAME) {
            SND_RenderMissAnimation();
        }
    } else {
        SND_OamHidePlaySlots();
        SND_OamHideTextSlots();
    }
}

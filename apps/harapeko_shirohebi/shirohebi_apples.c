/* Copyright (c) 2026 DAISUKE OBA. SPDX-License-Identifier: MIT.
   Game-specific apples operations. Functions below explain their state changes and hardware-
   facing responsibilities. */

/* Pomegranate placement and BG restoration. Apple identifiers name the fruit. */
static void SND_DrawAppleBg(u8 apple, u8 active);
static void SND_SpawnApple(u8 apple);
static u8 SND_AbsDiffU8(u8 a, u8 b);

/* Clear all candidate flags and activate the first permitted fruit position. */
static void SND_ResetApples()
{
    u8 i;

    i = 0;
    while (i < SND_APPLE_COUNT) {
        SND_AppleActive[(__safe_index u8)i] = 0;
        i = (u8)(i + 1);
    }
    SND_SpawnApple(0);
}

/* Convert a fruit center into a BG cell; draw its tile or restore the underlying map tile and
   attribute. */
static void SND_DrawAppleBg(u8 apple, u8 active)
{
    u8 x;
    u8 y;

    x = (u8)((SND_AppleX[(__safe_index u8)apple] - 4) >> 3);
    y = (u8)((SND_AppleY[(__safe_index u8)apple] - 4) >> 3);
    if (active != 0) {
        SND_SetCell(x, y, SND_TILE_APPLE, SND_BG_APPLE_ATTR);
    } else {
        SND_SetCell(x, y, SND_BaseTileAt(x, y), SND_BaseAttrAt(x, y));
    }
}

/* Convert an 8-pixel object's center to the tile containing its top-left corner. */
static u8 SND_TileFromCenter(u8 center)
{
    return (u8)((center - 4) >> 3);
}

/* Return the larger unsigned byte for the tile-distance calculation. */
static u8 SND_MaxU8(u8 a, u8 b)
{
    if (a > b) return a;
    return b;
}

/* Chebyshev distance in tiles produces square exclusion rings around mines. */
/* Return the smallest Chebyshev distance, in tiles, from this fruit candidate to either mine. */
static u8 SND_AppleBombDistance(u8 apple)
{
    u8 ax;
    u8 ay;
    u8 bx;
    u8 by;
    u8 dx;
    u8 dy;
    u8 dist;
    u8 best;
    u8 i;

    ax = SND_TileFromCenter(SND_AppleX[(__safe_index u8)apple]);
    ay = SND_TileFromCenter(SND_AppleY[(__safe_index u8)apple]);
    best = 255;
    i = 0;
    while (i < SND_BLOCK_COUNT) {
        bx = SND_TileFromCenter(SND_BlockX[(__safe_index u8)i]);
        by = SND_TileFromCenter(SND_BlockY[(__safe_index u8)i]);
        dx = SND_AbsDiffU8(ax, bx);
        dy = SND_AbsDiffU8(ay, by);
        dist = SND_MaxU8(dx, dy);
        if (dist < best) best = dist;
        i = (u8)(i + 1);
    }
    return best;
}

/* Admit progressively closer spawn rings at 10, 20 and 30 points. */
/* Choose the exclusion-ring radius from the score: 4, 3, 2 or 1 tiles at 0, 10, 20 and 30
   points. */
static u8 SND_AppleMinBombDistance()
{
    if (SND_ApplesEaten >= 30) return 1;
    if (SND_ApplesEaten >= 20) return 2;
    if (SND_ApplesEaten >= 10) return 3;
    return 4;
}

/* Reject a fruit candidate on a mine or inside the current score-dependent exclusion ring. */
static u8 SND_IsAppleAllowed(u8 apple)
{
    u8 dist;

    dist = SND_AppleBombDistance(apple);
    if (dist == 0) return 0;
    if (dist < SND_AppleMinBombDistance()) return 0;
    return 1;
}

/* Walk the fixed candidate cycle once at most, then activate exactly one fruit and draw it in
   the background. */
static void SND_SpawnApple(u8 apple)
{
    u8 i;
    u8 first;
    __safe_index u8 si;
    __safe_index u8 safe_apple;

    if (apple >= SND_APPLE_COUNT) apple = 0;
    /* Scan the fixed candidate cycle at most once, skipping unsafe mine rings. */
    first = apple;
    while (SND_IsAppleAllowed(apple) == 0) {
        apple = (u8)(apple + 1);
        if (apple >= SND_APPLE_COUNT) apple = 0;
        if (apple == first) break;
    }

    i = 0;
    while (i < SND_APPLE_COUNT) {
        si = i;
        SND_AppleActive[si] = 0;
        i = (u8)(i + 1);
    }
    SND_AppleIndex = apple;
    safe_apple = apple;
    SND_AppleActive[safe_apple] = 1;
    SND_ApplePosX = SND_AppleX[safe_apple];
    SND_ApplePosY = SND_AppleY[safe_apple];
    SND_DrawAppleBg(apple, 1);
}

/* Advance the candidate index with wraparound and let the spawn filter select a safe location. */
static void SND_SpawnNextApple()
{
    u8 next;

    next = (u8)(SND_AppleIndex + 1);
    if (next >= SND_APPLE_COUNT) next = 0;
    SND_SpawnApple(next);
}

/* Removing PAUSE/START must restore a live fruit before falling back to the map. */
/* Restore an active fruit before the base map when removing an overlay from a game cell. */
static void SND_RestoreGameCell(u8 x, u8 y)
{
    u8 i;
    u8 ax;
    u8 ay;

    i = 0;
    while (i < SND_APPLE_COUNT) {
        if (SND_AppleActive[(__safe_index u8)i] != 0) {
            ax = (u8)((SND_AppleX[(__safe_index u8)i] - 4) >> 3);
            ay = (u8)((SND_AppleY[(__safe_index u8)i] - 4) >> 3);
            if (ax == x && ay == y) {
                SND_SetCell(x, y, SND_TILE_APPLE, SND_BG_APPLE_ATTR);
                return;
            }
        }
        i = (u8)(i + 1);
    }

    SND_SetCell(x, y, SND_BaseTileAt(x, y), SND_BaseAttrAt(x, y));
}

#pragma bank 2

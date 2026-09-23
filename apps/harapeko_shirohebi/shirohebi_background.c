/* Copyright (c) 2026 DAISUKE OBA. SPDX-License-Identifier: MIT.
   Game-specific background operations. Functions below explain their state changes and
   hardware-facing responsibilities. */

/* Read a byte at a map base plus a 16-bit offset; the caller must keep the asset's ROM bank
   visible. */
static u8 SND_ReadAssetByte(u16 base, u16 off)
{
    u16 p;

    p = (u16)(base + off);
    return *(u8*)p;
}

/* The exported visible map is tightly packed at 20 cells per row, unlike the
   hardware tilemap's 32-cell stride handled by the tile-setting API. */
/* Compute a packed 20-column visible-map offset, not a hardware tilemap address. */
static u16 SND_MapIndex(u8 x, u8 y)
{
    return (u16)((u16)y * 20 + (u16)x);
}

/* Read the background tile number for one visible map cell. */
static u8 SND_BaseTileAt(u8 x, u8 y)
{
    return SND_ReadAssetByte((u16)shirohebi_map_tiles, SND_MapIndex(x, y));
}

/* Read the CGB attribute byte paired with one visible map cell. */
static u8 SND_BaseAttrAt(u8 x, u8 y)
{
    return SND_ReadAssetByte((u16)shirohebi_map_attrs, SND_MapIndex(x, y));
}

/* Keep the tile ID and CGB palette/attribute in sync for each partial update. */
/* Write a background tile and its CGB attribute together so partial redraws preserve their
   pairing. */
static void SND_SetCell(u8 x, u8 y, u8 tile, u8 attr)
{
    __settile(x, y, tile);
    __settileattr(x, y, attr);
}

/* Reconstruct the 20-by-18 visible playfield from the supplied map tables. */
static void SND_BuildBackground()
{
    u8 x;
    u8 y;

    y = 0;
    while (y < 18) {
        x = 0;
        while (x < 20) {
            SND_SetCell(x, y, SND_BaseTileAt(x, y), SND_BaseAttrAt(x, y));
            x = (u8)(x + 1);
        }
        y = (u8)(y + 1);
    }
}

#pragma bank 2

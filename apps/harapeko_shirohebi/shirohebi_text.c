/* Copyright (c) 2026 DAISUKE OBA. SPDX-License-Identifier: MIT.
   Game-specific text operations. Functions below explain their state changes and hardware-
   facing responsibilities. */

#pragma bank 3

static __prg_rom u8 SND_AsciiSymbols[SND_ASCII_SYMBOL_COUNT] = {
    0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28,
    0x29, 0x2A, 0x2B, 0x2C, 0x2D, 0x2E, 0x2F, 0x3A,
    0x3B, 0x3C, 0x3D, 0x3E, 0x3F, 0x40, 0x5B, 0x5C,
    0x5D, 0x5F, 0x60, 0x7B, 0x7D, 0x7E
};

/* Map supported uppercase, digits, lowercase and thirty symbols into the 92-glyph index space;
   return 0xFF for unsupported characters. */
static u8 SND_AsciiIndex(u8 c)
{
    u8 i;

    if (c >= 'A' && c <= 'Z') return (u8)(c - 'A');
    if (c >= '0' && c <= '9') return (u8)(26 + c - '0');
    if (c >= 'a' && c <= 'z') return (u8)(36 + c - 'a');

    i = 0;
    while (i < SND_ASCII_SYMBOL_COUNT) {
        if (c == SND_AsciiSymbols[(__safe_index u8)i]) return (u8)(62 + i);
        i = (u8)(i + 1);
    }
    return 0xFF;
}

/* New names use only A-Z and period; the wider glyph set remains available
   for UI text and previously saved names. */
/* Convert the restricted name-entry index into A-Z or period. */
static u8 SND_NameCharAt(u8 index)
{
    if (index < 26) return (u8)('A' + index);
    return '.';
}

/* Translate a character to its installed VRAM tile; unsupported characters and space select
   background restoration. */
static u8 SND_CharToTile(char c)
{
    u8 index;

    if (c == ' ') return 0;
    if (c >= 'A' && c <= 'Z') return (u8)(SND_TILE_FONT_BASE + (u8)(c - 'A'));
    if (c >= '0' && c <= '9') return (u8)(SND_TILE_REV_DIGIT + (u8)(c - '0'));
    if (c >= 'a' && c <= 'z') return (u8)(SND_TILE_ASCII_LOWER + (u8)(c - 'a'));

    index = SND_AsciiIndex((u8)c);
    if (index >= 62 && index < 92) {
        return (u8)(SND_TILE_ASCII_SYMBOL + index - 62);
    }
    return 0;
}

/* Draw a character with the floor palette, or restore the underlying map when no glyph tile is
   selected. */
static void SND_PutChar(u8 x, u8 y, char c)
{
    u8 tile;

    tile = SND_CharToTile(c);
    /* A space restores the underlying cell rather than painting a solid box. */
    if (tile == 0) {
        SND_SetCell(x, y, SND_BaseTileAt(x, y), SND_BaseAttrAt(x, y));
        return;
    }
    SND_SetCell(x, y, tile, KQ_CGB_ATTR_PAL(1));
}

/* Draw the menu cursor tile with the same palette as its text row. */
static void SND_PutMenuArrow(u8 x, u8 y)
{
    SND_SetCell(x, y, SND_TILE_ARROW, KQ_CGB_ATTR_PAL(1));
}

/* Write a zero-terminated string across consecutive background cells; callers supply positions
   and strings that fit the row. */
static void SND_Print(u8 x, u8 y, const char* text)
{
    while (*text != 0) {
        SND_PutChar(x, y, *text);
        x = (u8)(x + 1);
        text++;
    }
}

/* Restore all twenty cells in one visible text row through the normal space/background path. */
static void SND_ClearRow(u8 y)
{
    u8 x;

    x = 0;
    while (x < 20) {
        SND_PutChar(x, y, ' ');
        x = (u8)(x + 1);
    }
}

/* Clamp a digit to 0-9 and draw its corresponding character. */
static void SND_PutDigit(u8 x, u8 y, u8 digit)
{
    if (digit > 9) digit = 9;
    SND_PutChar(x, y, (char)('0' + digit));
}

/* Reserve three cells with right alignment, but blank leading zeroes.
   Repeated subtraction avoids a general division helper on the target CPU. */
/* Extract hundreds/tens by repeated subtraction and print three right-aligned cells with blank
   leading zeroes. */
static void SND_PrintScore3(u8 x, u8 y, u8 score)
{
    u8 original;
    u8 hundreds;
    u8 tens;

    original = score;
    hundreds = 0;
    while (score >= 100) {
        score = (u8)(score - 100);
        hundreds = (u8)(hundreds + 1);
    }

    tens = 0;
    while (score >= 10) {
        score = (u8)(score - 10);
        tens = (u8)(tens + 1);
    }

    if (original >= 100) SND_PutDigit(x, y, hundreds);
    else SND_PutChar(x, y, ' ');
    if (original >= 10) SND_PutDigit((u8)(x + 1), y, tens);
    else SND_PutChar((u8)(x + 1), y, ' ');
    SND_PutDigit((u8)(x + 2), y, score);
}

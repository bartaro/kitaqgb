/* Copyright (c) 2026 DAISUKE OBA. SPDX-License-Identifier: MIT.
   Game-specific display operations. Functions below explain their state changes and hardware-
   facing responsibilities. */

/* Clear all 40 shadow OAM entries and reset the sprite library's active/used bookkeeping. */
static void SND_OamClear()
{
    u8 i;

    i = 0;
    while (i < 40) {
        kq_sprite_oam[(__safe_index u8)i].y = 0;
        kq_sprite_oam[(__safe_index u8)i].x = 0;
        kq_sprite_oam[(__safe_index u8)i].tile = 0;
        kq_sprite_oam[(__safe_index u8)i].flags = 0;
        kq_sprite_active[(__safe_index u8)i] = 0;
        i = (u8)(i + 1);
    }
    kq_sprite_used = 0;
}

/* Shadow OAM uses hardware coordinates: logical top-left plus X=8, Y=16.
   Callers must not add these offsets themselves. */
/* Write a logical top-left position to shadow OAM with hardware X+8/Y+16 offsets, rejecting
   indices outside the 40-entry array. */
static void SND_OamSet(u8 index, u8 x, u8 y, u8 tile, u8 attr)
{
    if (index >= 40) return;
    kq_sprite_oam[(__safe_index u8)index].y = (u8)(y + 16);
    kq_sprite_oam[(__safe_index u8)index].x = (u8)(x + 8);
    kq_sprite_oam[(__safe_index u8)index].tile = tile;
    kq_sprite_oam[(__safe_index u8)index].flags = attr;
    kq_sprite_active[(__safe_index u8)index] = 1;
    if ((u8)(index + 1) > kq_sprite_used) kq_sprite_used = (u8)(index + 1);
}

/* Clear the 31 slots reserved for the snake so hidden game states cannot leave sprites on
   screen. */
static void SND_OamHidePlaySlots()
{
    u8 i;

    i = 0;
    while (i < SND_OAM_PLAY_USED) {
        kq_sprite_oam[(__safe_index u8)i].y = 0;
        kq_sprite_oam[(__safe_index u8)i].x = 0;
        kq_sprite_oam[(__safe_index u8)i].tile = 0;
        kq_sprite_oam[(__safe_index u8)i].flags = 0;
        kq_sprite_active[(__safe_index u8)i] = 0;
        i = (u8)(i + 1);
        SND_AudioServiceWork(i);
    }
}

/* Hide leftover slots after a shorter run; stale sprites still consume the
   scanline sprite budget even when no current joint is assigned to them. */
/* Clear snake slots beyond the current length; otherwise stale entries would still consume
   scanline sprite capacity. */
static void SND_OamHideUnusedPlaySlots(u8 first)
{
    u8 i;

    i = first;
    while (i < SND_OAM_PLAY_USED) {
        kq_sprite_oam[(__safe_index u8)i].y = 0;
        kq_sprite_oam[(__safe_index u8)i].x = 0;
        kq_sprite_oam[(__safe_index u8)i].tile = 0;
        kq_sprite_oam[(__safe_index u8)i].flags = 0;
        kq_sprite_active[(__safe_index u8)i] = 0;
        i = (u8)(i + 1);
        SND_AudioServiceWork(i);
    }
}

/* Clear the nine OAM slots reserved for text and other overlays. */
static void SND_OamHideTextSlots()
{
    u8 i;
    u8 oam;

    i = 0;
    while (i < SND_OAM_TEXT_COUNT) {
        oam = (u8)(SND_OAM_TEXT_BASE + i);
        kq_sprite_oam[(__safe_index u8)oam].y = 0;
        kq_sprite_oam[(__safe_index u8)oam].x = 0;
        kq_sprite_oam[(__safe_index u8)oam].tile = 0;
        kq_sprite_oam[(__safe_index u8)oam].flags = 0;
        kq_sprite_active[(__safe_index u8)oam] = 0;
        i = (u8)(i + 1);
        SND_AudioServiceWork(i);
    }
}

/* Clear the final reserved score slot and its active flag. */
static void SND_OamHideScoreTail()
{
    u8 oam;

    oam = (u8)(SND_OAM_TEXT_BASE + 8);
    kq_sprite_oam[(__safe_index u8)oam].y = 0;
    kq_sprite_oam[(__safe_index u8)oam].x = 0;
    kq_sprite_oam[(__safe_index u8)oam].tile = 0;
    kq_sprite_oam[(__safe_index u8)oam].flags = 0;
    kq_sprite_active[(__safe_index u8)oam] = 0;
}

/* Transfer the sprite library's shadow OAM to hardware; the main loop calls this after its
   VBlank wait. */
static void SND_OamCommit()
{
    sprite_flush_oam_now();
}

/* Assign the game's BG/OBJ palettes, including fruit, growth flash, impact and fade colors. */
static void SND_LoadPalettes()
{
    cgb_bg_palette(0, SND_BG_PAL_TEXT);
    cgb_bg_palette(1, SND_BG_PAL_FLOOR);
    cgb_bg_palette(2, SND_BG_PAL_DOT);
    cgb_bg_palette(3, SND_BG_PAL_COPYRIGHT_GOLD);
    cgb_bg_palette(4, SND_BG_PAL_COPYRIGHT_IVORY);
    cgb_bg_palette(5, SND_BG_PAL_APPLE);
    cgb_bg_palette(6, SND_BG_PAL_SCORCH);
    cgb_bg_palette(7, SND_BG_PAL_COPYRIGHT_PINK);
    cgb_obj_palette(0, SND_OBJ_PAL_SNAKE);
    cgb_obj_palette(1, SND_OBJ_PAL_APPLE);
    cgb_obj_palette(2, SND_OBJ_PAL_TEXT);
    cgb_obj_palette(3, SND_OBJ_PAL_HIT);
    cgb_obj_palette(4, SND_OBJ_PAL_GROW);
    cgb_obj_palette(5, SND_OBJ_PAL_FADE_LIGHT);
    cgb_obj_palette(6, SND_OBJ_PAL_FADE_DARK);
    cgb_obj_palette(7, SND_OBJ_PAL_FADE_BLUE);
}

/* Upload exported background art, copy the banked 24-tile snake/effect block and construct the
   text tiles. */
static void SND_LoadTiles()
{
    shirohebi_upload_tiles();
    __far_memcpy((void*)0x8380, __bankof(SND_SPRITE_TILES), SND_SPRITE_TILES, 384);
    SND_LoadTextTiles();
}

/* Wait for VBlank, disable the LCD for bulk VRAM setup, initialize sprite state and restore
   the configured display. */
static void SND_InitDisplay()
{
    system_wait_vblank();
    /* Bulk tile/map uploads happen with the LCD off; restore VRAM bank zero
       before loading shared DMG/CGB sprite data. */
    SND_LCDC = 0;
    SND_BGP = 0xE4;
    /* Map the snake's body colors to DMG white while retaining dark eye pixels. */
    SND_OBP0 = 0x0C;
    SND_OBP1 = 0xE4;

    __cgb_safe_set_vbk(0);
    __vram_fill(0x8000, 0, 4096);
    __vram_fill(0x9800, 0, 1024);
    __cgb_safe_set_vbk(1);
    __vram_fill(0x9800, 0, 1024);
    __cgb_safe_set_vbk(0);

    sprite_init();
    SND_OamClear();
    SND_OamCommit();
    SND_LoadTiles();
    SND_LoadPalettes();
    SND_BuildBackground();
    SND_LCDC = 0x93;
}

/* Redraw or clear the title prompt only when its requested visibility changes. */
static void SND_SetTitlePromptVisible(u8 visible)
{
    u8 x;

    if (visible == SND_TitlePromptShown) return;
    SND_TitlePromptShown = visible;
    if (visible != 0) {
        SND_Print(SND_TITLE_PROMPT_X, SND_TITLE_PROMPT_Y, "PRESS START BUTTON");
        return;
    }

    x = SND_TITLE_PROMPT_X;
    while (x < (u8)(SND_TITLE_PROMPT_X + SND_TITLE_PROMPT_LEN)) {
        SND_PutChar(x, SND_TITLE_PROMPT_Y, ' ');
        x = (u8)(x + 1);
    }
}

/* Advance the title blink counter and derive visibility from bit 6. */
static void SND_UpdateTitlePrompt()
{
    u8 visible;

    SND_TitlePromptTimer = (u8)(SND_TitlePromptTimer + 1);
    visible = 1;
    if ((SND_TitlePromptTimer & SND_TITLE_BLINK_MASK) != 0) visible = 0;
    SND_SetTitlePromptVisible(visible);
}

/* Each 16x16 glyph is four consecutive tiles: TL, TR, BL, BR. */
/* Lay out eight 16-by-16 title glyphs using four consecutive 8-by-8 tiles per glyph. */
static void SND_DrawJapaneseTitle()
{
    u8 i;
    u8 tile;
    u8 x;

    i = 0;
    while (i < 8) {
        tile = (u8)(SND_TILE_JP_TITLE + (i << 2));
        x = (u8)(2 + (i << 1));
        SND_SetCell(x, 1, tile, KQ_CGB_ATTR_PAL(1));
        SND_SetCell((u8)(x + 1), 1, (u8)(tile + 1), KQ_CGB_ATTR_PAL(1));
        SND_SetCell(x, 2, (u8)(tile + 2), KQ_CGB_ATTR_PAL(1));
        SND_SetCell((u8)(x + 1), 2, (u8)(tile + 3), KQ_CGB_ATTR_PAL(1));
        i = (u8)(i + 1);
    }
}

/* Draw the 19 notice tiles, applying the gold, ivory and pink palette groups on CGB. */
static void SND_DrawCopyright()
{
    u8 i;

    i = 0;
    while (i < SND_COPYRIGHT_TILE_COUNT) {
        u8 attr;

        if (i < 5) attr = KQ_CGB_ATTR_PAL(3);
        else if (i < 10) attr = KQ_CGB_ATTR_PAL(4);
        else attr = KQ_CGB_ATTR_PAL(7);
        SND_SetCell((u8)(SND_COPYRIGHT_X + i), SND_COPYRIGHT_Y,
                    (u8)(SND_TILE_COPYRIGHT_BASE + i), attr);
        i = (u8)(i + 1);
    }
}

/* Replace one mine's background cell with floor when a menu must cover it. */
static void SND_SetMineCellToFloor(u8 mine_index)
{
    u8 x;
    u8 y;

    x = (u8)(SND_BlockX[(__safe_index u8)mine_index] >> 3);
    y = (u8)(SND_BlockY[(__safe_index u8)mine_index] >> 3);
    SND_SetCell(x, y, 0, KQ_CGB_ATTR_PAL(1));
}

/* Spaces restore the base map, so blank ranking cells over mine locations
   need a final floor tile instead of revealing those base-map mines. */
/* Cover mine cells that would otherwise show through blank ranking text on the title screen. */
static void SND_HideUncoveredTitleMines()
{
    if (SND_HighScores[1] < 100) SND_SetMineCellToFloor(0);
    if (SND_HighScoreNames[18] == ' ') SND_SetMineCellToFloor(1);
}

/* Rebuild the background, title, ranking, copyright and sound options, then reset the prompt
   blink and idle timer. */
static void SND_DrawTitle()
{
    u8 row;

    SND_BuildBackground();
    row = 3;
    while (row < 18) {
        SND_ClearRow(row);
        row = (u8)(row + 1);
    }
    SND_DrawJapaneseTitle();
    SND_Print(1, 3, "HARAPEKO SHIROHEBI");
    SND_DrawHighScoreArea();
    SND_DrawCopyright();
    SND_DrawTitleMenuRow(SND_TITLE_MENU_MUSIC);
    SND_DrawTitleMenuRow(SND_TITLE_MENU_SOUND);
    SND_HideUncoveredTitleMines();
    SND_TitlePromptTimer = 0;
    SND_TitlePromptShown = 0;
    SND_TitleIdleFrames = 0;
    /* Do not release the input lock while transition buttons are still held. */
    SND_SetTitlePromptVisible(1);
}

/* Draw the attract-mode indicator in the HUD row. */
static void SND_DrawDemoLabel()
{
    SND_Print(SND_DEMO_LABEL_X, SND_DEMO_LABEL_Y, "DEMO");
}

/* Move only the two YES/NO arrow cells; preserve the erase dialog's static labels. */
static void SND_DrawEraseChoice()
{
    /* Only the old and new arrow cells change. ERASE?, NO and YES stay put. */
    if (SND_EraseChoice == SND_ERASE_YES) {
        SND_PutChar(4, 9, ' ');
        SND_PutMenuArrow(10, 9);
    } else {
        SND_PutChar(10, 9, ' ');
        SND_PutMenuArrow(4, 9);
    }
}

/* Construct the erase confirmation once on entry and cover mine cells beneath it. */
static void SND_DrawEraseDialog()
{
    u8 row;

    /* Full dialog setup runs once, on entry, never on selection changes. */
    row = 4;
    while (row < 16) {
        SND_ClearRow(row);
        row = (u8)(row + 1);
    }
    SND_Print(7, 7, "ERASE?");
    SND_Print(6, 9, "NO   YES");
    SND_DrawEraseChoice();
    SND_SetMineCellToFloor(0);
    SND_SetMineCellToFloor(1);
}

/* Draw one explosion tile only when its coordinates lie inside the visible 20-by-18 map. */
static void SND_DrawMineBlastCell(u8 x, u8 y, u8 tile)
{
    if (x >= 20 || y >= 18) return;
    SND_SetCell(x, y, tile, KQ_CGB_ATTR_PAL(5));
}

/* Animate the impacted mine's cross-shaped five-cell blast, alternating flash and burst tiles. */
static void SND_DrawMineExplosionFrame()
{
    u8 cx;
    u8 cy;
    u8 tile;

    if (SND_CollisionKind != SND_COLLISION_MINE) return;
    if (SND_HitMineIndex >= SND_BLOCK_COUNT) return;

    cx = (u8)(SND_BlockX[(__safe_index u8)SND_HitMineIndex] >> 3);
    cy = (u8)(SND_BlockY[(__safe_index u8)SND_HitMineIndex] >> 3);
    tile = SND_TILE_BURST;
    if ((SND_MissTimer & 4) != 0) tile = SND_TILE_FLASH;

    SND_DrawMineBlastCell(cx, cy, tile);
    if (cx != 0) SND_DrawMineBlastCell((u8)(cx - 1), cy, tile);
    SND_DrawMineBlastCell((u8)(cx + 1), cy, tile);
    if (cy != 0) SND_DrawMineBlastCell(cx, (u8)(cy - 1), tile);
    SND_DrawMineBlastCell(cx, (u8)(cy + 1), tile);
}

/* Reapply the crater after rebuilding the base map on result/name screens. */
/* Restore a crater at the impacted mine after a result screen rebuilds the map. */
static void SND_DrawMineAftermath()
{
    u8 cx;
    u8 cy;

    if (SND_CollisionKind != SND_COLLISION_MINE) return;
    if (SND_HitMineIndex >= SND_BLOCK_COUNT) return;

    cx = (u8)(SND_BlockX[(__safe_index u8)SND_HitMineIndex] >> 3);
    cy = (u8)(SND_BlockY[(__safe_index u8)SND_HitMineIndex] >> 3);

    SND_SetCell(cx, cy, SND_TILE_SCORCH, SND_BG_SCORCH_ATTR);
}

/* Selection changes never clear or redraw the TRY AGAIN labels. */
/* Move only the retry prompt's selection arrow, leaving its labels intact. */
static void SND_DrawGameOverChoice()
{
    if (SND_GameOverChoice == SND_GAMEOVER_NO) {
        SND_PutChar(4, 11, ' ');
        SND_PutMenuArrow(11, 11);
    } else {
        SND_PutChar(11, 11, ' ');
        SND_PutMenuArrow(4, 11);
    }
}

/* Draw static labels only when entering the screen. */
/* Prepare the retry choice row once, then draw the current YES/NO cursor. */
static void SND_DrawGameOverOptions()
{
    SND_ClearRow(11);
    SND_Print(5, 11, "YES    NO");
    SND_DrawGameOverChoice();
}

/* Rebuild the result screen with the crater, GAME OVER label and retry prompt. */
static void SND_DrawGameOver()
{
    SND_BuildBackground();
    SND_DrawMineAftermath();
    SND_Print(5, 6, "GAME OVER");
    SND_Print(5, 9, "TRY AGAIN?");
    SND_DrawGameOverOptions();
}

/* After name confirmation, show only the retry prompt, not GAME OVER again. */
/* Draw the retry-only screen after name confirmation, preserving the crater if applicable. */
static void SND_DrawTryAgain()
{
    SND_BuildBackground();
    SND_DrawMineAftermath();
    SND_Print(5, 9, "TRY AGAIN?");
    SND_DrawGameOverOptions();
}

/* Redraw the selected character of the pending ranking name, if an insertion is active. */
static void SND_DrawNameEntryChar()
{
    u8 offset;
    u8 x;

    if (SND_PendingRank == SND_PENDING_NONE) return;
    offset = (u8)(SND_PendingRank + SND_PendingRank + SND_PendingRank + SND_NameCursor);
    x = (u8)(8 + SND_NameCursor + SND_NameCursor);
    SND_PutChar(x, 12, (char)SND_HighScoreNames[(__safe_index u8)offset]);
}

/* Clear the three cursor cells and mark the selected name position without clearing its
   letters. */
static void SND_DrawNameEntryCursor()
{
    u8 i;

    i = 0;
    while (i < SND_NAME_LENGTH) {
        SND_PutChar((u8)(7 + i + i), 12, ' ');
        i = (u8)(i + 1);
    }
    SND_PutMenuArrow((u8)(7 + SND_NameCursor + SND_NameCursor), 12);
}

/* Build the ranked-result screen, draw all three name characters and return the cursor to the
   first character. */
static void SND_DrawNameEntry()
{
    u8 i;

    SND_BuildBackground();
    SND_DrawMineAftermath();
    SND_Print(5, 5, "GAME OVER");
    SND_Print(6, 8, "RANK");
    SND_PrintRank2(11, 8, (u8)(SND_PendingRank + 1));
    SND_Print(8, 10, "NAME");

    i = 0;
    while (i < SND_NAME_LENGTH) {
        SND_NameCursor = i;
        SND_DrawNameEntryChar();
        i = (u8)(i + 1);
    }
    SND_NameCursor = 0;
    SND_DrawNameEntryCursor();
}

/* Restore the unused score cells and print the current byte-sized score without leading
   zeroes. */
static void SND_DrawHud()
{
    u8 x;

    x = 3;
    while (x < 9) {
        SND_SetCell(x, 0, SND_BaseTileAt(x, 0), SND_BaseAttrAt(x, 0));
        x = (u8)(x + 1);
    }
    SND_PrintScore3(0, 0, SND_ApplesEaten);
}

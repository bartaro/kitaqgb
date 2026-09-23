/* Copyright (c) 2026 DAISUKE OBA. SPDX-License-Identifier: MIT.
   Game-specific save operations. Functions below explain their state changes and hardware-
   facing responsibilities. */

#pragma bank 3

/* Print ranks 1-10 in two right-aligned cells, reserving a blank tens cell for single-digit
   ranks. */
static void SND_PrintRank2(u8 x, u8 y, u8 rank)
{
    if (rank >= 10) {
        SND_PutDigit(x, y, 1);
        SND_PutDigit((u8)(x + 1), y, 0);
    } else {
        SND_PutChar(x, y, ' ');
        SND_PutDigit((u8)(x + 1), y, rank);
    }
}

/* Print the three name bytes belonging to one ranking entry. */
static void SND_PrintHighScoreName(u8 x, u8 y, u8 rank_index)
{
    u8 i;
    u8 offset;

    offset = (u8)(rank_index + rank_index + rank_index);
    i = 0;
    while (i < SND_NAME_LENGTH) {
        SND_PutChar((u8)(x + i), y,
                    (char)SND_HighScoreNames[(__safe_index u8)(offset + i)]);
        i = (u8)(i + 1);
    }
}

/* Split ranks 1-5 and 6-10 into fixed columns; score cells stay right-aligned. */
/* Draw ranks 1-5 and 6-10 in two columns, keeping names and scores associated. */
static void SND_PrintHighScores()
{
    u8 i;
    u8 row;
    u8 right;

    i = 0;
    while (i < 5) {
        row = (u8)(7 + i);
        SND_PrintRank2(0, row, (u8)(i + 1));
        SND_PrintHighScoreName(3, row, i);
        SND_PrintScore3(6, row, SND_HighScores[(__safe_index u8)i]);

        right = (u8)(i + 5);
        SND_PrintRank2(10, row, (u8)(right + 1));
        SND_PrintHighScoreName(13, row, right);
        SND_PrintScore3(16, row, SND_HighScores[(__safe_index u8)right]);
        i = (u8)(i + 1);
    }
}

/* Clear the ranking rows, print their heading and redraw all ten entries. */
static void SND_DrawHighScoreArea()
{
    u8 row;

    row = 5;
    while (row < 12) {
        SND_ClearRow(row);
        row = (u8)(row + 1);
    }
    SND_Print(6, 5, "RANKING");
    SND_PrintHighScores();
}

/* Draw a fixed-width ON/OFF value so a shorter ON cannot leave a stale final letter. */
static void SND_PrintOnOff(u8 x, u8 y, u8 on)
{
    if (on != 0) SND_Print(x, y, "ON ");
    else SND_Print(x, y, "OFF");
}

/* Construct one option row with its label, current value and optional selection arrow. */
static void SND_DrawTitleOption(u8 index, u8 y, const char* label, u8 on)
{
    SND_ClearRow(y);
    if (SND_TitleMenuPos == index) {
        SND_PutMenuArrow(4, y);
    }
    SND_Print(6, y, label);
    SND_PrintOnOff(12, y, on);
}

static void SND_SaveScores();

/* Route a menu index to the MUSIC or SOUND option row. */
static void SND_DrawTitleMenuRow(u8 index)
{
    if (index == SND_TITLE_MENU_MUSIC) {
        SND_DrawTitleOption(SND_TITLE_MENU_MUSIC, 13, "MUSIC", SND_MusicOn);
    } else if (index == SND_TITLE_MENU_SOUND) {
        SND_DrawTitleOption(SND_TITLE_MENU_SOUND, 14, "SOUND", SND_SoundOn);
    }
}

/* Menu navigation updates only the two cursor cells, including SELECT. */
/* Update only the two menu arrow cells after a selection change. */
static void SND_DrawTitleMenuCursor()
{
    if (SND_TitleMenuPos == SND_TITLE_MENU_MUSIC) {
        SND_PutChar(4, 14, ' ');
        SND_PutMenuArrow(4, 13);
    } else {
        SND_PutChar(4, 13, ' ');
        SND_PutMenuArrow(4, 14);
    }
}

/* Redraw only the selected option's ON/OFF field after toggling it. */
static void SND_DrawTitleMenuValue()
{
    if (SND_TitleMenuPos == SND_TITLE_MENU_MUSIC) SND_PrintOnOff(12, 13, SND_MusicOn);
    else SND_PrintOnOff(12, 14, SND_SoundOn);
}

/* Accept older saved glyphs even though new name entry has a smaller alphabet. */
/* Accept space or any glyph supported by the game font when validating a stored name. */
static u8 SND_IsValidNameChar(u8 c)
{
    if (c == ' ') return 1;
    return (u8)(SND_AsciiIndex(c) != 0xFF);
}

/* Initialize all ten scores to zero and all thirty name bytes to hyphens. */
static void SND_ResetScoreTable()
{
    u8 i;

    i = 0;
    while (i < SND_HISCORE_COUNT) {
        SND_HighScores[(__safe_index u8)i] = 0;
        i = (u8)(i + 1);
    }

    i = 0;
    while (i < SND_NAME_BYTES) {
        SND_HighScoreNames[(__safe_index u8)i] = '-';
        i = (u8)(i + 1);
    }
}

/* Read and validate the 42-byte versioned payload; import supported shorter score layouts or
   initialize SRAM when no valid payload exists. */
static void SND_LoadScores()
{
    u8 i;
    u8 c;
    u16 save_len;

    SND_ResetScoreTable();

    /* Missing or invalid SRAM starts all ten ranks at zero, not legacy data. */
    if (save_exists(0) == 0) {
        SND_SaveScores();
        return;
    }

    /* Prefer the versioned ten-score/three-character-name payload. The save
       library validates its outer header, exact length and checksum first. */
    save_len = SND_SAVE_DATA_BYTES;
    if (save_read(0, SND_SaveData, save_len) != 0) {
        if (SND_SaveData[0] == SND_SAVE_MAGIC && SND_SaveData[1] == SND_SAVE_VERSION) {
            i = 0;
            while (i < SND_HISCORE_COUNT) {
                SND_HighScores[(__safe_index u8)i] =
                    SND_SaveData[(__safe_index u8)(SND_SAVE_SCORE_OFFSET + i)];
                i = (u8)(i + 1);
            }
            i = 0;
            while (i < SND_NAME_BYTES) {
                c = SND_SaveData[(__safe_index u8)(SND_SAVE_NAME_OFFSET + i)];
                if (SND_IsValidNameChar(c) == 0) c = '-';
                SND_HighScoreNames[(__safe_index u8)i] = c;
                i = (u8)(i + 1);
            }
            return;
        }
    }

    /* Older releases stored five scores alone, or three five-score mode tables.
       Import the first five entries and leave the remaining ranks at zero. */
    save_len = SND_OLD_SCORE_COUNT;
    if (save_read(0, SND_LegacyScores, save_len) != 0) {
        i = 0;
        while (i < (u8)SND_OLD_SCORE_COUNT) {
            SND_HighScores[(__safe_index u8)i] = SND_LegacyScores[(__safe_index u8)i];
            i = (u8)(i + 1);
        }
        SND_SaveScores();
        return;
    }

    save_len = SND_LEGACY_SCORE_BYTES;
    if (save_read(0, SND_LegacyScores, save_len) != 0) {
        i = 0;
        while (i < (u8)SND_OLD_SCORE_COUNT) {
            SND_HighScores[(__safe_index u8)i] = SND_LegacyScores[(__safe_index u8)i];
            i = (u8)(i + 1);
        }
    }
    SND_SaveScores();
}

/* Pack bytes explicitly so the SRAM format does not depend on struct padding. */
/* Pack magic, version, ten scores and thirty name bytes explicitly, then write slot zero
   through the save library. */
static void SND_SaveScores()
{
    u8 i;
    u16 save_len;

    SND_SaveData[0] = SND_SAVE_MAGIC;
    SND_SaveData[1] = SND_SAVE_VERSION;
    i = 0;
    while (i < SND_HISCORE_COUNT) {
        SND_SaveData[(__safe_index u8)(SND_SAVE_SCORE_OFFSET + i)] =
            SND_HighScores[(__safe_index u8)i];
        i = (u8)(i + 1);
    }
    i = 0;
    while (i < SND_NAME_BYTES) {
        SND_SaveData[(__safe_index u8)(SND_SAVE_NAME_OFFSET + i)] =
            SND_HighScoreNames[(__safe_index u8)i];
        i = (u8)(i + 1);
    }

    save_len = SND_SAVE_DATA_BYTES;
    save_write(0, SND_SaveData, save_len);
}

/* Reset the in-memory ranking and immediately store the cleared table to SRAM. */
static void SND_ClearScores()
{
    SND_ResetScoreTable();
    SND_SaveScores();
}

/* Stage an insertion in RAM; name confirmation commits it to SRAM.
   Zero is unranked, and equal scores stay behind existing equal entries. */
/* Insert a nonzero qualifying score in descending order, shift names with scores and stage AAA
   for editing; ties follow existing entries. */
static void SND_RecordScore()
{
    u8 i;
    u8 j;
    u8 k;
    u8 score;
    u8 dst;
    u8 src;

    SND_PendingRank = SND_PENDING_NONE;
    score = SND_ApplesEaten;
    if (score == 0) return;

    i = 0;
    while (i < SND_HISCORE_COUNT) {
        if (score > SND_HighScores[(__safe_index u8)i]) {
            /* Shift from the bottom, keeping each score and its name together. */
            j = (u8)(SND_HISCORE_COUNT - 1);
            while (j > i) {
                SND_HighScores[(__safe_index u8)j] =
                    SND_HighScores[(__safe_index u8)(j - 1)];
                dst = (u8)(j + j + j);
                src = (u8)((j - 1) + (j - 1) + (j - 1));
                k = 0;
                while (k < SND_NAME_LENGTH) {
                    SND_HighScoreNames[(__safe_index u8)(dst + k)] =
                        SND_HighScoreNames[(__safe_index u8)(src + k)];
                    k = (u8)(k + 1);
                }
                j = (u8)(j - 1);
            }

            SND_HighScores[(__safe_index u8)i] = score;
            dst = (u8)(i + i + i);
            k = 0;
            while (k < SND_NAME_LENGTH) {
                SND_HighScoreNames[(__safe_index u8)(dst + k)] = 'A';
                SND_NameIndices[(__safe_index u8)k] = 0;
                k = (u8)(k + 1);
            }
            SND_NameCursor = 0;
            SND_PendingRank = i;
            return;
        }
        i = (u8)(i + 1);
    }
}

/* Cycle the selected name character through 26 uppercase letters and period, updating both the
   index and staged ranking byte. */
static void SND_ChangePendingNameChar(u8 forward)
{
    u8 index;
    u8 offset;

    if (SND_PendingRank == SND_PENDING_NONE) return;
    index = SND_NameIndices[(__safe_index u8)SND_NameCursor];
    if (forward != 0) {
        index = (u8)(index + 1);
        if (index >= SND_NAME_CHAR_COUNT) index = 0;
    } else {
        if (index == 0) index = (u8)(SND_NAME_CHAR_COUNT - 1);
        else index = (u8)(index - 1);
    }
    SND_NameIndices[(__safe_index u8)SND_NameCursor] = index;
    offset = (u8)(SND_PendingRank + SND_PendingRank + SND_PendingRank + SND_NameCursor);
    SND_HighScoreNames[(__safe_index u8)offset] =
        SND_NameCharAt(index);
}
#pragma bank 3

/* Copyright (c) 2026 DAISUKE OBA. SPDX-License-Identifier: MIT.
   Game-specific pause operations. Functions below explain their state changes and hardware-
   facing responsibilities. */

/* Overlay requests are separated from VRAM writes. The main loop applies only
   dirty visibility changes after its VBlank wait, preserving the playfield. */
/* Write the five-character PAUSE overlay at its fixed background position. */
static void SND_DrawPauseText()
{
    SND_Print(SND_PAUSE_X, SND_PAUSE_Y, "PAUSE");
}

/* Restore each PAUSE cell using the active fruit and underlying map instead of painting a flat
   rectangle. */
static void SND_ClearPauseText()
{
    u8 i;

    i = 0;
    while (i < SND_PAUSE_LEN) {
        SND_RestoreGameCell((u8)(SND_PAUSE_X + i), SND_PAUSE_Y);
        i = (u8)(i + 1);
    }
}

/* Normalize the requested visibility and mark a deferred redraw only when needed. */
static void SND_SetPauseVisible(u8 visible)
{
    if (visible != 0) visible = 1;
    if (visible == SND_PauseDesired && SND_PauseDirty == 0) return;
    SND_PauseDesired = visible;
    SND_PauseDirty = 1;
}

/* Write the ready-screen START overlay at its fixed background position. */
static void SND_DrawStartText()
{
    SND_Print(SND_START_LABEL_X, SND_START_LABEL_Y, "START");
}

/* Restore the map/fruit cells occupied by the START overlay. */
static void SND_ClearStartText()
{
    u8 i;

    i = 0;
    while (i < SND_START_LABEL_LEN) {
        SND_RestoreGameCell((u8)(SND_START_LABEL_X + i), SND_START_LABEL_Y);
        i = (u8)(i + 1);
    }
}

/* Queue a normalized START visibility request without writing VRAM immediately. */
static void SND_SetStartVisible(u8 visible)
{
    if (visible != 0) visible = 1;
    if (visible == SND_StartDesired && SND_StartDirty == 0) return;
    SND_StartDesired = visible;
    SND_StartDirty = 1;
}

/* After the presentation wait, apply a dirty START visibility change and clear the request
   flag. */
static void SND_ApplyStartVisible()
{
    if (SND_StartDirty == 0) return;
    if (SND_StartDesired != SND_StartShown) {
        if (SND_StartDesired != 0) {
            SND_DrawStartText();
            SND_StartShown = 1;
        } else {
            SND_ClearStartText();
            SND_StartShown = 0;
        }
    }
    SND_StartDirty = 0;
}

/* After the presentation wait, apply a dirty PAUSE visibility change and clear the request
   flag. */
static void SND_ApplyPauseVisible()
{
    if (SND_PauseDirty == 0) return;
    if (SND_PauseDesired != SND_PauseShown) {
        if (SND_PauseDesired != 0) {
            SND_DrawPauseText();
            SND_PauseShown = 1;
        } else {
            SND_ClearPauseText();
            SND_PauseShown = 0;
        }
    }
    SND_PauseDirty = 0;
}

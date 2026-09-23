/* Copyright (c) 2026 DAISUKE OBA. SPDX-License-Identifier: MIT.
   Game-specific flow operations. Functions below explain their state changes and hardware-
   facing responsibilities. */

/* Handle idle-demo entry, game start and MUSIC/SOUND selection; redraw only changed menu
   values or cursors. */
static void SND_TickTitle(u8 keys, u8 trigger)
{
    u8 cursor_changed;
    u8 row_changed;

    cursor_changed = 0;
    row_changed = 0;

    if (keys != 0) {
        SND_TitleIdleFrames = 0;
    } else {
        if (SND_TitleIdleFrames < SND_TITLE_DEMO_WAIT_FRAMES) {
            SND_TitleIdleFrames = (u16)(SND_TitleIdleFrames + 1);
        }
        if (SND_TitleIdleFrames >= SND_TITLE_DEMO_WAIT_FRAMES) {
            SND_SetTitlePromptVisible(0);
            SND_StartDemo();
            return;
        }
    }

    /* SELECT+START has priority in SND_HandleSystemInput. Ordinary START
       and A are immediate again; no four-button gesture needs to assemble. */
    if ((trigger & PAD_KEY_START) != 0) {
        SND_StartGame();
        return;
    }

    if ((trigger & PAD_KEY_UP) != 0) {
        if (SND_TitleMenuPos == 0) SND_TitleMenuPos = (u8)(SND_TITLE_MENU_COUNT - 1);
        else SND_TitleMenuPos = (u8)(SND_TitleMenuPos - 1);
        cursor_changed = 1;
    } else if ((trigger & (PAD_KEY_DOWN | PAD_KEY_SELECT)) != 0) {
        /* SELECT cycles MUSIC -> SOUND -> MUSIC, once per press. */
        SND_TitleMenuPos = (u8)(SND_TitleMenuPos + 1);
        if (SND_TitleMenuPos >= SND_TITLE_MENU_COUNT) SND_TitleMenuPos = 0;
        cursor_changed = 1;
    }

    if ((trigger & (PAD_KEY_LEFT | PAD_KEY_RIGHT | PAD_KEY_A)) != 0) {
        if (SND_TitleMenuPos == SND_TITLE_MENU_MUSIC) {
            SND_AudioSetMusic((u8)(SND_MusicOn == 0));
            row_changed = 1;
        } else if (SND_TitleMenuPos == SND_TITLE_MENU_SOUND) {
            SND_AudioSetSound((u8)(SND_SoundOn == 0));
            row_changed = 1;
        }
    }
    if (cursor_changed != 0) SND_DrawTitleMenuCursor();
    if (row_changed != 0) SND_DrawTitleMenuValue();
    SND_UpdateTitlePrompt();
}

/* Give cancel precedence, toggle the choice on a fresh press and clear SRAM only after an
   explicit YES confirmation. */
static void SND_TickEraseConfirm(u8 trigger)
{
    /* Cancellation wins over a conflicting confirm press. */
    if ((trigger & (PAD_KEY_B | PAD_KEY_START)) != 0) {
        SND_ReturnToTitle();
        return;
    }
    if ((trigger & (PAD_KEY_LEFT | PAD_KEY_RIGHT | PAD_KEY_SELECT)) != 0) {
        if (SND_EraseChoice == SND_ERASE_YES) SND_EraseChoice = SND_ERASE_NO;
        else SND_EraseChoice = SND_ERASE_YES;
        SND_DrawEraseChoice();
        return;
    }
    if ((trigger & PAD_KEY_A) != 0) {
        if (SND_EraseChoice == SND_ERASE_YES) SND_ClearScores();
        SND_ReturnToTitle();
    }
}

/* Advance collision animation, then leave a demo or open name entry/retry with a fresh-input
   requirement. */
static void SND_TickMiss(u8 trigger)
{
    trigger = trigger;

    /* Self-bites reuse the frozen pose for fading; only mine hits scatter it. */
    if (SND_CollisionKind != SND_COLLISION_SELF &&
        SND_MissTimer >= SND_MISS_FLASH_FRAMES &&
        SND_MissTimer < SND_MISS_CLEAR_FRAME) {
        SND_UpdateMissPieces();
    }
    if (SND_MissTimer < SND_MISS_CLEAR_FRAME) {
        SND_DrawMineExplosionFrame();
    }
    if (SND_MissTimer == SND_MISS_CLEAR_FRAME) {
        if (SND_DemoActive != 0) {
            SND_EndDemo();
            return;
        }
        /* Require a fresh press after the miss animation, on either screen. */
        SND_LockSystemButtons();
        SND_GameOverChoice = SND_GAMEOVER_YES;
        SND_AudioPlayGameOverMusic();
        if (SND_PendingRank != SND_PENDING_NONE) {
            SND_DrawNameEntry();
            SND_State = SND_STATE_NAME_ENTRY;
        } else {
            SND_DrawGameOver();
            SND_State = SND_STATE_GAMEOVER;
        }
        return;
    }
    if (SND_MissTimer < 250) {
        SND_MissTimer = (u8)(SND_MissTimer + 1);
    }
}

/* Commit the staged ranking to SRAM, clear the pending insertion and open the retry prompt
   without carrying the confirm press. */
static void SND_FinishNameEntry()
{
    /* Do not carry the finishing A/START or a held SELECT into TRY AGAIN. */
    SND_LockSystemButtons();
    SND_SaveScores();
    SND_PendingRank = SND_PENDING_NONE;
    SND_GameOverChoice = SND_GAMEOVER_YES;
    SND_DrawTryAgain();
    SND_State = SND_STATE_GAMEOVER;
}

/* Edit A-Z/period, move among three character positions and confirm with START or A on the
   final position; SELECT only moves the cursor. */
static void SND_TickNameEntry(u8 trigger)
{
    if ((trigger & PAD_KEY_UP) != 0) {
        SND_ChangePendingNameChar(1);
        SND_DrawNameEntryChar();
        return;
    }
    if ((trigger & PAD_KEY_DOWN) != 0) {
        SND_ChangePendingNameChar(0);
        SND_DrawNameEntryChar();
        return;
    }

    if ((trigger & PAD_KEY_LEFT) != 0 ||
        ((trigger & PAD_KEY_B) != 0 && SND_NameCursor != 0)) {
        if (SND_NameCursor == 0) SND_NameCursor = (u8)(SND_NAME_LENGTH - 1);
        else SND_NameCursor = (u8)(SND_NameCursor - 1);
        SND_DrawNameEntryCursor();
        return;
    }
    /* SELECT advances one character position, like RIGHT; the last
       position wraps to the first and never confirms the name. */
    if ((trigger & (PAD_KEY_RIGHT | PAD_KEY_SELECT)) != 0) {
        SND_NameCursor = (u8)(SND_NameCursor + 1);
        if (SND_NameCursor >= SND_NAME_LENGTH) SND_NameCursor = 0;
        SND_DrawNameEntryCursor();
        return;
    }

    if ((trigger & PAD_KEY_START) != 0) {
        SND_FinishNameEntry();
        return;
    }
    if ((trigger & PAD_KEY_A) != 0) {
        if (SND_NameCursor >= (u8)(SND_NAME_LENGTH - 1)) {
            SND_FinishNameEntry();
        } else {
            SND_NameCursor = (u8)(SND_NameCursor + 1);
            SND_DrawNameEntryCursor();
        }
    }
}

/* Toggle the retry choice and route confirmation to a new run or the title. */
static void SND_TickGameOver(u8 trigger)
{
    if ((trigger & (PAD_KEY_LEFT | PAD_KEY_RIGHT | PAD_KEY_SELECT)) != 0) {
        if (SND_GameOverChoice == SND_GAMEOVER_YES) {
            SND_GameOverChoice = SND_GAMEOVER_NO;
        } else {
            SND_GameOverChoice = SND_GAMEOVER_YES;
        }
        SND_DrawGameOverChoice();
        return;
    }

    if ((trigger & (PAD_KEY_A | PAD_KEY_START)) != 0) {
        if (SND_GameOverChoice == SND_GAMEOVER_YES) {
            SND_StartGame();
        } else {
            SND_ReturnToTitle();
        }
    }
}

/* Blink START during the introduction, then enable movement and select demo timing or the main
   song. */
static void SND_TickReady(u8 trigger)
{
    u8 phase;
    u8 visible;

    trigger = trigger;

    visible = 0;
    if (SND_StartTimer < (u8)(SND_START_FLASH_CYCLE * 3)) {
        phase = SND_StartTimer;
        while (phase >= SND_START_FLASH_CYCLE) {
            phase = (u8)(phase - SND_START_FLASH_CYCLE);
        }
        if (phase < SND_START_FLASH_ON) visible = 1;
    }
    SND_SetStartVisible(visible);

    /* Start the demo timeout only when movement starts, excluding the intro. */
    if (SND_StartTimer >= SND_START_FLASH_TOTAL) {
        SND_SetStartVisible(0);
        SND_State = SND_STATE_PLAY;
        if (SND_DemoActive != 0) {
            SND_ResetDemoClock();
            SND_AudioStopBgm();
            SND_DrawDemoLabel();
        } else {
            SND_AudioStartRunMusic();
        }
        return;
    }

    SND_StartTimer = (u8)(SND_StartTimer + 1);
}

/* Freeze gameplay, reset steering repeat, request the pause overlay, pause audio and require
   button release. */
static void SND_EnterPause()
{
    SND_LockSystemButtons();
    SND_TurnRepeatDir = SND_STEER_NONE;
    SND_TurnRepeatTimer = 0;
    SND_PauseTimer = 0;
    SND_PauseShown = 0;
    SND_SetPauseVisible(1);
    SND_AudioPause(1);
    SND_State = SND_STATE_PAUSE;
}

/* Remove the pause overlay and resume audio/play only after rearming input through the release
   gate. */
static void SND_LeavePause()
{
    SND_LockSystemButtons();
    SND_TurnRepeatDir = SND_STEER_NONE;
    SND_TurnRepeatTimer = 0;
    SND_SetPauseVisible(0);
    SND_AudioPause(0);
    SND_State = SND_STATE_PLAY;
}

/* Prioritize START to resume; a SELECT press opens title confirmation; otherwise update the
   blinking overlay. */
static void SND_TickPause(u8 trigger)
{
    u8 visible;

    if ((trigger & PAD_KEY_START) != 0) {
        SND_LeavePause();
        return;
    }
    /* Open on one fresh press, not a timed hold. The IRQ latch also catches
       a short tap while main is blocked. Entry consumes this press so it
       cannot immediately toggle the prompt's initial NO selection. */
    if ((trigger & PAD_KEY_SELECT) != 0 || SND_PauseSelectPending != 0) {
        SND_OpenReturnConfirm();
        return;
    }

    SND_PauseTimer = (u8)(SND_PauseTimer + 1);
    visible = 1;
    if ((SND_PauseTimer & 16) != 0) visible = 0;
    SND_SetPauseVisible(visible);
}

/* Demos replace controller input, not physics or collision rules. */
/* Apply speed and steering, integrate the head, wrap/round it, update the chain, test fatal
   collisions before fruit and expire growth highlighting. */
static void SND_TickPlay(u8 keys, u8 trigger)
{
    if (SND_DemoActive != 0) {
        if (SND_DemoClockExpired() != 0) {
            SND_EndDemo();
            return;
        }
        keys = SND_DemoControlKeys();
        trigger = 0;
    }

    if ((trigger & PAD_KEY_START) != 0) {
        SND_EnterPause();
        return;
    }

    SND_UpdateSpeedForKeys(keys);
    SND_ApplySteering(keys);
    SND_AudioServiceWork(0);
    kq2d_step(&SND_World);
    SND_AudioServiceWork(0);
    SND_WrapHead();
    SND_SyncHeadPixel();
    SND_AudioServiceWork(0);
    /* Test the updated pose. A fatal hit takes precedence over eating fruit. */
    SND_UpdateArticulatedBody();
    SND_AudioServiceWork(0);
    if (SND_CheckBlockMiss() != 0) {
        SND_WallMiss();
        return;
    }
    SND_AudioServiceWork(0);
    if (SND_CheckSelfMiss() != 0) {
        SND_WallMiss();
        return;
    }
    SND_AudioServiceWork(0);
    SND_CheckApple();
    if (SND_GrowFlashTimer != 0) {
        SND_GrowFlashTimer = (u8)(SND_GrowFlashTimer - 1);
        if (SND_GrowFlashTimer == 0) {
            SND_GrowFlashSegment = SND_SEGMENT_NONE;
        }
    }
}


/* Keep shortcut handling and screen dispatch in one place. Tests include this
   exact function, so shortcut precedence cannot silently diverge from main. */
/* Process global shortcuts/release locks first, consume demo-exit input, then run exactly the
   handler for the current game state. */
static void SND_DispatchInput(u8 keys, u8 trigger)
{
    if (SND_HandleSystemInput(keys) != 0) return;

    /* Consume a demo-exit press instead of applying it to the title. */
    if (SND_DemoActive != 0 && keys != 0) {
        SND_EndDemo();
    } else if (SND_State == SND_STATE_TITLE) {
        SND_TickTitle(keys, trigger);
    } else if (SND_State == SND_STATE_READY) {
        SND_TickReady(trigger);
    } else if (SND_State == SND_STATE_PLAY) {
        SND_TickPlay(keys, trigger);
    } else if (SND_State == SND_STATE_PAUSE) {
        SND_TickPause(trigger);
    } else if (SND_State == SND_STATE_ERASE) {
        SND_TickEraseConfirm(trigger);
    } else if (SND_State == SND_STATE_RETURN_CONFIRM) {
        SND_TickReturnConfirm(trigger);
    } else if (SND_State == SND_STATE_GAMEOVER) {
        SND_TickGameOver(trigger);
    } else if (SND_State == SND_STATE_NAME_ENTRY) {
        SND_TickNameEntry(trigger);
    } else {
        SND_TickMiss(trigger);
    }
}

/* Copyright (c) 2026 DAISUKE OBA. SPDX-License-Identifier: MIT.
   Game-specific input operations. Functions below explain their state changes and hardware-
   facing responsibilities. */

#pragma bank 3

/* Disarm before clearing pending events so an ISR cannot requeue an event
   halfway through a screen transition. Every shared event flag is 8-bit. */
/* Disarm interrupt-side shortcut capture before clearing its pending events to prevent
   requeueing during a transition. */
static void SND_DisableInputShortcuts()
{
    SND_TitleShortcutEnabled = 0;
    SND_PauseSelectEnabled = 0;
    SND_TitleShortcutPending = 0;
    SND_PauseSelectPending = 0;
}

/* Disable shortcut capture and require all buttons to be released before the next screen
   accepts commands. */
static void SND_LockSystemButtons()
{
    SND_DisableInputShortcuts();
    SND_InputReleaseLock = 1;
}

/* Abandon a run without submitting a score. Preserve the existing save format,
   MUSIC/SOUND preferences, and the saved ranking table. */
/* Abandon the run without committing a pending score, stop effects, clear overlays/sprites and
   rebuild the title while retaining options. */
static void SND_ReturnToTitle()
{
    SND_AudioStopBgm();
    Audio_StopSfx();
    Audio_SetPaused(0);
    SND_DemoActive = 0;
    SND_ResetDemoClock();
    if (SND_PendingRank != SND_PENDING_NONE) {
        SND_LoadScores();
        SND_PendingRank = SND_PENDING_NONE;
    }

    SND_Length = 0;
    SND_MissTimer = 0;
    SND_UpBoostHeld = 0;
    SND_TurnRepeatDir = SND_STEER_NONE;
    SND_TurnRepeatTimer = 0;
    SND_GrowFlashSegment = SND_SEGMENT_NONE;
    SND_GrowFlashTimer = 0;

    /* Drop queued gameplay overlays BEFORE drawing the title. Clearing a
       pending START/PAUSE overlay afterwards would overwrite title tiles. */
    SND_StartTimer = 0;
    SND_StartShown = 0;
    SND_StartDesired = 0;
    SND_StartDirty = 0;
    SND_PauseTimer = 0;
    SND_PauseShown = 0;
    SND_PauseDesired = 0;
    SND_PauseDirty = 0;
    SND_LockSystemButtons();
    SND_OamClear();
    system_wait_vblank();
    SND_OamCommit();
    SND_State = SND_STATE_TITLE;
    SND_DrawTitle();
}

/* The return prompt touches just two background rows. Cancel reconstructs
   those cells with live fruit state; it never resets the snake or the run. */
/* Move only the return-confirmation arrows between NO and YES. */
static void SND_DrawReturnChoice()
{
    if (SND_ReturnChoice == SND_RETURN_YES) {
        SND_PutChar(4, 9, ' ');
        SND_PutMenuArrow(10, 9);
    } else {
        SND_PutChar(10, 9, ' ');
        SND_PutMenuArrow(4, 9);
    }
}

/* Consume the opening press, default to NO, hide sprites and replace two background rows with
   the return prompt. */
static void SND_OpenReturnConfirm()
{
    SND_LockSystemButtons();
    SND_ReturnChoice = SND_RETURN_NO;
    SND_SetPauseVisible(0);
    SND_ApplyPauseVisible();
    SND_State = SND_STATE_RETURN_CONFIRM;
    SND_OamClear();
    system_wait_vblank();
    SND_OamCommit();
    SND_ClearRow(7);
    SND_ClearRow(9);
    SND_Print(3, 7, "BACK TO TITLE?");
    SND_Print(6, 9, "NO   YES");
    SND_DrawReturnChoice();
}

/* Reconstruct the affected rows from live fruit/base-map state and return to PAUSE without
   resetting the run or unpausing audio. */
static void SND_CancelReturnConfirm()
{
    u8 x;

    SND_LockSystemButtons();
    x = 0;
    while (x < 20) {
        SND_RestoreGameCell(x, 7);
        SND_RestoreGameCell(x, 9);
        x = (u8)(x + 1);
    }
    SND_PauseTimer = 0;
    SND_SetPauseVisible(1);
    SND_State = SND_STATE_PAUSE;
    /* Audio remains paused; only a fresh START press resumes the run. */
}

/* Prioritize cancel, handle one-press choice toggles and either abandon the run or restore the
   paused screen. */
static void SND_TickReturnConfirm(u8 trigger)
{
    if ((trigger & (PAD_KEY_B | PAD_KEY_START)) != 0) {
        SND_CancelReturnConfirm();
        return;
    }
    if ((trigger & (PAD_KEY_LEFT | PAD_KEY_RIGHT | PAD_KEY_SELECT)) != 0) {
        if (SND_ReturnChoice == SND_RETURN_YES) SND_ReturnChoice = SND_RETURN_NO;
        else SND_ReturnChoice = SND_RETURN_YES;
        SND_DrawReturnChoice();
        return;
    }
    if ((trigger & PAD_KEY_A) != 0) {
        if (SND_ReturnChoice == SND_RETURN_YES) SND_ReturnToTitle();
        else SND_CancelReturnConfirm();
    }
}

/* Called before ordinary controls. No four-key requirement remains.
   A release lock also consumes the release frame, preventing a carried A,
   START, or direction from confirming/toggling something on the new screen. */
/* Resolve the optional four-button shortcut, release gate and latched title shortcut before
   ordinary state input; arm pause SELECT capture only when allowed. */
static u8 SND_HandleSystemInput(u8 keys)
{
    /* Optional legacy hold for controllers that can deliver four buttons.
       Check gameplay before the pause-entry lock: START may have arrived
       first. On the title the new SELECT+START rule covers this combination.
       Never turn a four-button hold into YES/NO input inside either dialog. */
    if ((keys & SND_COMMAND_BUTTONS) == SND_COMMAND_BUTTONS) {
        if (SND_State == SND_STATE_ERASE || SND_State == SND_STATE_RETURN_CONFIRM) {
            SND_LockSystemButtons();
            return 1;
        }
        if (SND_State != SND_STATE_TITLE) {
            SND_ReturnToTitle();
            return 1;
        }
    }
    if (SND_InputReleaseLock != 0) {
        SND_DisableInputShortcuts();
        if (keys == 0) {
            SND_InputReleaseLock = 0;
            if (SND_State == SND_STATE_TITLE && SND_DemoActive == 0) {
                SND_TitleShortcutEnabled = 1;
            }
            if (SND_State == SND_STATE_PAUSE && SND_DemoActive == 0) {
                SND_PauseSelectEnabled = 1;
            }
        }
        return 1;
    }

    if (SND_State == SND_STATE_TITLE && SND_DemoActive == 0) {
        SND_TitleShortcutEnabled = 1;
        if (SND_TitleShortcutPending != 0 ||
            (keys & SND_TITLE_SHORTCUT) == SND_TITLE_SHORTCUT) {
            SND_LockSystemButtons();
            SND_EraseChoice = SND_ERASE_NO;
            SND_State = SND_STATE_ERASE;
            SND_SetTitlePromptVisible(0);
            SND_DrawEraseDialog();
            return 1;
        }
    } else {
        SND_TitleShortcutEnabled = 0;
        SND_TitleShortcutPending = 0;
    }

    if (SND_State == SND_STATE_PAUSE && SND_DemoActive == 0) {
        SND_PauseSelectEnabled = 1;
    } else {
        SND_PauseSelectEnabled = 0;
        SND_PauseSelectPending = 0;
    }
    return 0;
}

#pragma bank 2

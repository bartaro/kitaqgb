/* Copyright (c) 2026 DAISUKE OBA. SPDX-License-Identifier: MIT.
   Game-specific main operations. Functions below explain their state changes and hardware-
   facing responsibilities. */

/* Initialize hardware-facing and game state, install the fixed-bank audio hook, load ranking
   data and repeatedly update input, state and VBlank presentation. */
void main()
{
    u16 kt;
    u8 keys;
    u8 trigger;

    system_init();
    SND_IsCgb = __cgb_is_cgb();
    SND_InitDisplay();
    SND_State = SND_STATE_TITLE;
    SND_PrevKeys = 0;
    SND_Length = 0;
    SND_AppleIndex = 0;
    SND_MissTimer = 0;
    SND_ApplesEaten = 0;
    SND_Facing = SND_DIR_RIGHT;
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
    SND_TitleMenuPos = SND_TITLE_MENU_MUSIC;
    SND_TitlePromptTimer = 0;
    SND_TitlePromptShown = 0;
    SND_TitleIdleFrames = 0;
    SND_DemoActive = 0;
    SND_DemoScene = 1;
    SND_DemoWeaveStep = 0;
    SND_DemoSeconds = 0;
    SND_DemoSubTicks = 0;
    SND_DemoVBlankCounter = 0;
    SND_DemoVBlankPrevious = 0;
    SND_MusicOn = 1;
    SND_SoundOn = 1;
    SND_UpBoostHeld = 0;
    SND_EraseChoice = SND_ERASE_NO;
    SND_SystemButtonsSample = 0;
    SND_TitleShortcutEnabled = 0;
    SND_TitleShortcutPending = 0;
    SND_PauseSelectEnabled = 0;
    SND_PauseSelectPending = 0;
    SND_InputReleaseLock = 1;
    SND_ReturnChoice = SND_RETURN_NO;
    SND_GameOverChoice = SND_GAMEOVER_YES;
    SND_PendingRank = SND_PENDING_NONE;
    SND_NameCursor = 0;
    SND_HitMineIndex = SND_MINE_NONE;
    SND_CurrentMaxVelQ4 = SND_BASE_MAX_VEL_Q4;
    SND_GrowFlashSegment = SND_SEGMENT_NONE;
    SND_GrowFlashTimer = 0;
    SND_AudioInit();
    /* Install the fixed-bank callback only after all shared state is initialized. */
    AudioVBlank_FrameHook = SND_DemoVBlankHook;
    AudioVBlank_EnableIrq();
    SND_LoadScores();
    SND_DrawTitle();

    while (1) {
        SND_AudioUpdate();
        /* Low byte: held buttons; high byte: newly pressed buttons this tick. */
        kt = __readpadex(SND_PrevKeys);
        SND_PrevKeys = (u8)kt;
        keys = (u8)kt;
        trigger = (u8)(kt >> 8);

        SND_DispatchInput(keys, trigger);

        /* Prepare shadow OAM first, then commit after the VBlank wait.
           Overlay tile writes are deferred to this presentation phase as well. */
        SND_RenderSprites();
        system_wait_vblank();
        SND_OamCommit();
        SND_AudioForceUpdate();
        SND_ApplyStartVisible();
        SND_ApplyPauseVisible();
    }
}

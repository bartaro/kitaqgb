/* Copyright (c) 2026 DAISUKE OBA. SPDX-License-Identifier: MIT.
   Game-specific audio operations. Functions below explain their state changes and hardware-
   facing responsibilities. */

#pragma bank 3

/* Music records advance in the fixed-bank VBlank ISR. This module only routes
   game state/options and services standard-driver SFX from the main loop. */
extern __prg_rom u8 SND_SFX_CRASH[];
extern __prg_rom u8 SND_SFX_MINE_EXPLODE[];
extern __prg_rom u8 SND_SFX_EAT[];
extern __prg_rom u8 SND_VB_GAMESTART_BGM[];
extern __prg_rom u8 SND_VB_BGM[];
extern __prg_rom u8 SND_VB_GAMEOVER_BGM[];

/* Legacy scanline-clock bookkeeping; this flag no longer advances the music. */
static u8 SND_AudioVBlankArmed;

/* Start the looping main song through the fixed-bank VBlank music driver. */
static void SND_AudioPlayMusic()
{
    AudioVBlank_PlayMusic(SND_VB_BGM);
}

/* Start the finite ready-screen jingle from its resident music record array. */
static void SND_AudioPlayGameStartMusic()
{
    AudioVBlank_PlayMusic(SND_VB_GAMESTART_BGM);
}

/* Clear both the ISR song and standard-driver music state on transitions. */
/* Stop both music-driver states so a screen transition cannot leave either song active. */
static void SND_AudioStopBgm()
{
    AudioVBlank_Stop();
    Audio_StopMusic();
}

/* Return whether the current state permits the main song; title and result screens select
   their own music policy. */
static u8 SND_AudioShouldPlayMusic()
{
    if (SND_State == SND_STATE_PLAY) return 1;
    if (SND_State == SND_STATE_PAUSE) return 1;
    return 0;
}

/* Enable music and clear pause state, then start the main song only during play or pause. */
static void SND_AudioStartStateMusic()
{
    if (SND_MusicOn == 0) return;

    AudioVBlank_SetEnabled(1);
    Audio_SetMusicEnabled(1);
    Audio_SetPaused(0);
    Audio_StopMusic();

    if (SND_State == SND_STATE_PLAY || SND_State == SND_STATE_PAUSE) {
        SND_AudioPlayMusic();
    }
}

/* Normalize the menu option, stop both song states when disabled, and restart permitted state
   music when enabled. */
static void SND_AudioSetMusic(u8 on)
{
    SND_MusicOn = (u8)(on != 0);
    if (SND_MusicOn == 0) {
        AudioVBlank_SetEnabled(0);
        AudioVBlank_Stop();
        Audio_SetMusicEnabled(0);
        Audio_StopMusic();
        return;
    }

    AudioVBlank_SetEnabled(1);
    Audio_SetMusicEnabled(1);
    if (SND_AudioShouldPlayMusic() != 0) {
        SND_AudioStartStateMusic();
    }
}

/* Normalize the sound-effects option and forward it to the standard driver's SFX gate. */
static void SND_AudioSetSound(u8 on)
{
    SND_SoundOn = (u8)(on != 0);
    Audio_SetSfxEnabled(SND_SoundOn);
}

/* Pass the game's pause state to the shared audio driver; do not restart or replace the song. */
static void SND_AudioPause(u8 on)
{
    Audio_SetPaused(on);
}

/* Request the banked self-collision effect at priority 8 when effects are enabled. */
static void SND_AudioPlayCrash()
{
    if (SND_SoundOn == 0) return;
    Audio_PlaySFXBanked(__bankof(SND_SFX_CRASH), SND_SFX_CRASH, 8);
}

/* Trigger an immediate CH4 noise attack, then schedule the banked mine effect at priority 9. */
static void SND_AudioPlayMineExplosion()
{
    if (SND_SoundOn == 0) return;
    /* Give the mine an immediate noise attack before the banked pitch sweep. */
    Audio_Ch4NoteOn(0x53, 0xF6);
    Audio_PlaySFXBanked(__bankof(SND_SFX_MINE_EXPLODE), SND_SFX_MINE_EXPLODE, 9);
}

/* Request the banked fruit-eating effect at priority 8 when effects are enabled. */
static void SND_AudioPlayEat()
{
    if (SND_SoundOn == 0) return;
    Audio_PlaySFXBanked(__bankof(SND_SFX_EAT), SND_SFX_EAT, 8);
}

/* Unpause audio and start the result-screen song only when the MUSIC option is enabled. */
static void SND_AudioPlayGameOverMusic()
{
    Audio_SetPaused(0);
    Audio_StopMusic();
    if (SND_MusicOn != 0) {
        AudioVBlank_SetEnabled(1);
        Audio_SetMusicEnabled(1);
        AudioVBlank_PlayMusic(SND_VB_GAMEOVER_BGM);
    }
}

/* Refresh scanline bookkeeping; music timing itself is owned by the VBlank interrupt. */
static void SND_AudioResetStableClock()
{
    if (SND_LY >= SND_AUDIO_VBLANK_LINE) SND_AudioVBlankArmed = 0;
    else SND_AudioVBlankArmed = 1;
}

/* Mark main-loop audio bookkeeping as active without decoding another music record. */
static void SND_AudioUpdate()
{
    SND_AudioVBlankArmed = 1;
}

/* Main-loop SFX/driver maintenance; the separate ISR owns BGM timing. */
/* Service SFX and standard-driver maintenance once after presentation; the ISR advances the
   music separately. */
static void SND_AudioForceUpdate()
{
    SND_AudioVBlankArmed = 0;
    Audio_Update();
}

/* Intentionally empty: body/render loops must not advance audio per joint. */
/* Accept a work-loop progress value without advancing audio. Calling this from each joint must
   not accelerate music or effects. */
static void SND_AudioServiceWork(u8 step)
{
    step = step;
}

/* Initialize both driver layers, install the resident song pointer and apply the initial
   MUSIC/SOUND options. */
static void SND_AudioInit()
{
    SND_AudioVBlankArmed = 1;
    Audio_Init();
    AudioVBlank_Init();
    AudioVBlank_SetMusic(SND_VB_BGM);
    AudioVBlank_SetEnabled(SND_MusicOn);
    Audio_SetMusicEnabled(SND_MusicOn);
    Audio_SetSfxEnabled(SND_SoundOn);
    Audio_StopMusic();
}

/* Reset audio state for active play and begin the main song when music is enabled. */
static void SND_AudioStartRunMusic()
{
    SND_AudioResetStableClock();
    Audio_SetPaused(0);
    AudioVBlank_SetEnabled(SND_MusicOn);
    Audio_SetMusicEnabled(SND_MusicOn);
    AudioVBlank_Stop();
    Audio_StopMusic();
    if (SND_MusicOn != 0) {
        SND_AudioPlayMusic();
    }
}

/* Reset audio state for READY and begin its short introduction when music is enabled. */
static void SND_AudioStartGameJingle()
{
    SND_AudioResetStableClock();
    Audio_SetPaused(0);
    AudioVBlank_SetEnabled(SND_MusicOn);
    Audio_SetMusicEnabled(SND_MusicOn);
    AudioVBlank_Stop();
    Audio_StopMusic();
    if (SND_MusicOn != 0) {
        SND_AudioPlayGameStartMusic();
    }
}

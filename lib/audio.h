#pragma once

// Shared Game Boy audio driver for KITAQGB projects.
//
// Integration notes:
// - Compile a hardware-register file before audio.c so NR10..NR52 and
//   WAVE0..WAVE15 are already declared.
// - Call Audio_Init() once during startup.
// - Call Audio_Update() once per frame, ideally from your VBlank-driven main loop.
// - audio.c is intended to stay in bank 0.
// - Music streams are banked explicitly with Audio_PlayMusic(bank, song).
// - Audio_PlaySFX(sfx, priority) captures the currently visible ROM bank.
// - Audio_PlaySFXBanked(bank, sfx, priority) safely classifies CH1/CH3 data
//   after switching to the requested bank, so banked SFX can be started from
//   any currently visible ROM bank.
// - GBDAW-Stream v1.2 extends the legacy command set with explicit per-channel
//   automation commands that map more directly from GUI and Python tooling.

#define AUDIO_CHANNEL_CH1 ((u8)0)
#define AUDIO_CHANNEL_CH2 ((u8)1)
#define AUDIO_CHANNEL_CH3 ((u8)2)
#define AUDIO_CHANNEL_CH4 ((u8)3)

// Legacy music-stream IDs used by AUDIO_CMD_NOTE / AUDIO_CMD_SET_INST.
#define AUDIO_STREAM_CHANNEL_CH1 ((u8)0)
#define AUDIO_STREAM_CHANNEL_CH2 ((u8)1)
#define AUDIO_STREAM_CHANNEL_CH4 ((u8)2)
#define AUDIO_STREAM_CHANNEL_CH3 ((u8)3)

#define AUDIO_PAN_CENTER ((u8)0)
#define AUDIO_PAN_LEFT ((u8)1)
#define AUDIO_PAN_RIGHT ((u8)2)
#define AUDIO_PAN_MUTE ((u8)3)
#define AUDIO_PAN_INHERIT ((u8)0xFF)

#define AUDIO_WAVE_TRIANGLE ((u8)0)
#define AUDIO_WAVE_SAW ((u8)1)
#define AUDIO_WAVE_VOWEL_A ((u8)2)
#define AUDIO_WAVE_VOWEL_I ((u8)3)
#define AUDIO_WAVE_VOWEL_U ((u8)4)
#define AUDIO_WAVE_VOWEL_E ((u8)5)
#define AUDIO_WAVE_VOWEL_O ((u8)6)
#define AUDIO_WAVE_CUSTOM ((u8)7)

#define AUDIO_CH3_LEVEL_MUTE ((u8)0x00)
#define AUDIO_CH3_LEVEL_100 ((u8)0x20)
#define AUDIO_CH3_LEVEL_50 ((u8)0x40)
#define AUDIO_CH3_LEVEL_25 ((u8)0x60)

#define AUDIO_VOLUME_MIN ((u8)0)
#define AUDIO_VOLUME_MAX ((u8)7)
#define AUDIO_NOTE_MAX ((u8)67)

#define AUDIO_CMD_WAIT ((u8)0x00)
#define AUDIO_CMD_NOTE ((u8)0x10)
#define AUDIO_CMD_SET_INST ((u8)0x11)
#define AUDIO_CMD_SET_PAN ((u8)0x12)
#define AUDIO_CMD_SET_SWEEP ((u8)0x13)
#define AUDIO_CMD_SET_WAVE ((u8)0x14)
#define AUDIO_CMD_SET_CH3_CUSTOM_WAVE ((u8)0x15)
#define AUDIO_CMD_SET_CH4_PARAM ((u8)0x16)
#define AUDIO_CMD_SET_MASTER_VOLUME ((u8)0x17)
#define AUDIO_CMD_SET_CH1_DUTY ((u8)0x18)
#define AUDIO_CMD_SET_CH2_DUTY ((u8)0x19)
#define AUDIO_CMD_SET_CH3_LEVEL ((u8)0x1A)
#define AUDIO_CMD_STOP_CHANNEL ((u8)0x20)
#define AUDIO_CMD_LOOP ((u8)0x30)
#define AUDIO_CMD_STOP ((u8)0x40)

#define AUDIO_SFX_CH3_MARKER ((u8)0xFF)

void Audio_Init();
void Audio_Update();
void __stackcall Audio_PlayMusic(u8 bank, u8 *song);
void Audio_StopMusic();
void __stackcall Audio_SetPaused(u8 on);
void __stackcall Audio_PlaySFX(u8 *sfx, u8 priority);
void __stackcall Audio_PlaySFXBanked(u8 bank, u8 *sfx, u8 priority);
void __stackcall Audio_PlaySFXPanned(u8 *sfx, u8 priority, u8 pan);
void __stackcall Audio_PlaySFXPannedBanked(u8 bank, u8 *sfx, u8 priority, u8 pan);
void Audio_StopSfx();
void __stackcall Audio_StopChannel(u8 ch);

void __stackcall Audio_SetMusicEnabled(u8 on);
void __stackcall Audio_SetSfxEnabled(u8 on);

void __stackcall Audio_SetMasterVolume(u8 left, u8 right);
void __stackcall Audio_FadeToMasterVolume(u8 left, u8 right, u8 step_frames);
void Audio_CancelMasterVolumeFade();

void __stackcall Audio_SetPan(u8 ch, u8 pan);
// Two bits per channel (CH1 in bits 0-1): center, left, right, mute.
// Applies all four positions with one NR51 update for sequencer automation.
void __stackcall Audio_SetPanPacked(u8 packed);
void __stackcall Audio_LoadWave(u8 wave_id);
void __stackcall Audio_LoadCustomWave(const u8 *wave16);

void __stackcall Audio_SetCh1Duty(u8 duty);
void __stackcall Audio_SetCh2Duty(u8 duty);
void __stackcall Audio_SetCh3Level(u8 level);
void __stackcall Audio_SetCh4Param(u8 param);

void __stackcall Audio_Ch1NoteOn(u8 note, u8 vol_env, u8 duty2);
void __stackcall Audio_Ch2NoteOn(u8 note, u8 vol_env, u8 duty2);
void __stackcall Audio_Ch3NoteOn(u8 note, u8 level);
void __stackcall Audio_Ch4NoteOn(u8 param, u8 vol_env);

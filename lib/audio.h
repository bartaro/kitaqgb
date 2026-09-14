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

// Physical channel IDs differ from legacy stream IDs below: CH3 and CH4 exchange positions.
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

// Enable the APU and reset software streams, instruments, mix and fade state.
// Load the initial triangle wave, then leave every channel silent. Call at startup
// before concurrent interrupt-driven audio service can access these shared fields.
void Audio_Init();
// Advance the master fade first. Unless paused, service both effect streams
// then the optional legacy music decoder. Call once per frame for frame-based timing.
void Audio_Update();
// Retain the bank and song pointer, reset legacy instruments/cursors, and arm
// playback from the beginning. Supply a valid, persistent song; no bytes are copied
// and null is not checked. Existing effect streams remain active.
void __stackcall Audio_PlayMusic(u8 bank, u8 *song);
// Stop legacy music and clear its resume mask, then silence all physical channels.
// Effect pointers remain active and may produce sound on a later update.
void Audio_StopMusic();
// On pause entry, silence hardware while retaining cursors. On resume, retrigger
// eligible legacy CH1, CH2 and CH4 notes; CH3 is not retriggered here. Updates still
// advance master fades while paused, and effect playback resumes on a later update.
void __stackcall Audio_SetPaused(u8 on);
// Capture the currently mapped ROM bank and start an effect with inherited pan.
// Use the explicit-bank API when the pointer belongs to another bank.
void __stackcall Audio_PlaySFX(u8 *sfx, u8 priority);
// Start a banked effect with the channel's base pan, subject to priority arbitration.
void __stackcall Audio_PlaySFXBanked(u8 bank, u8 *sfx, u8 priority);
// Capture the currently mapped ROM bank and request an effect with a pan override.
// Use the explicit-bank API when the pointer belongs to another bank.
void __stackcall Audio_PlaySFXPanned(u8 *sfx, u8 priority, u8 pan);
// Retain a banked effect stream if its priority equals or exceeds the current
// owner's priority, or that slot is empty. A leading CH3 marker selects the wave
// slot and is skipped; other streams use the pulse slot. Paused requests are ignored.
// Keep stream storage valid until completion; acceptance does not trigger a note.
void __stackcall Audio_PlaySFXPannedBanked(u8 bank, u8 *sfx, u8 priority, u8 pan);
// Clear both effect streams and their pan overrides. When not paused, release
// borrowed channels through the configured legacy or VBlank recovery path.
void Audio_StopSfx();
// Stop a physical channel and clear its legacy resume bit. CH1 also clears the
// pulse effect stream; CH3 clears the wave effect stream. This is the legacy
// channel API and does not clear a VBlank driver's independently latched notes.
void __stackcall Audio_StopChannel(u8 ch);

// Set the music-processing gate. This does not silence notes already playing
// or stop the legacy stream cursor from consuming commands.
void __stackcall Audio_SetMusicEnabled(u8 on);
// Enable effect note writes, or disable them and release both effect streams.
// Disabling follows Audio_StopSfx recovery rules rather than silencing all channels.
void __stackcall Audio_SetSfxEnabled(u8 on);

// Clamp both output levels to 0..7, cancel any fade, and write NR50 immediately.
void __stackcall Audio_SetMasterVolume(u8 left, u8 right);
// Move left/right volume toward clamped targets at the requested Audio_Update
// call interval. An interval of zero applies the levels immediately.
void __stackcall Audio_FadeToMasterVolume(u8 left, u8 right, u8 step_frames);
// Cancel further volume steps, retaining the current cached levels and hardware output.
void Audio_CancelMasterVolumeFade();

// Update the base pan for a physical channel and reapply active effect overrides.
// Unknown channel IDs select CH4; unsupported pan values mute the selected channel.
void __stackcall Audio_SetPan(u8 ch, u8 pan);
// Two bits per channel (CH1 in bits 0-1): center, left, right, mute.
// Applies all four positions with one NR51 update for sequencer automation.
void __stackcall Audio_SetPanPacked(u8 packed);
// Install a named preset or the cached custom wave. Unknown IDs are ignored.
// This does not update Audio_Ch3Wave or trigger a new note.
void __stackcall Audio_LoadWave(u8 wave_id);
// Copy 16 packed sample bytes into the persistent custom cache and wave RAM.
// A null pointer is ignored; the source is needed only for this call.
void __stackcall Audio_LoadCustomWave(const u8 *wave16);

// Cache the low two duty bits for the next CH1 note; do not change the active register.
void __stackcall Audio_SetCh1Duty(u8 duty);
// Cache the low two duty bits for the next CH2 note; do not change the active register.
void __stackcall Audio_SetCh2Duty(u8 duty);
// Cache the raw NR32 level encoding for the next CH3 note without writing hardware.
void __stackcall Audio_SetCh3Level(u8 level);
// Cache the raw noise polynomial encoding for the next CH4 note without writing hardware.
void __stackcall Audio_SetCh4Param(u8 param);

// Trigger CH1 with the cached sweep and the supplied raw envelope. Clamp the
// note index; duty values 0, 1 and 3 select their encodings, and all others use 2.
void __stackcall Audio_Ch1NoteOn(u8 note, u8 vol_env, u8 duty2);
// Trigger CH2 with the supplied raw envelope and a clamped note index. Duty
// values 0, 1 and 3 select their encodings; all other values use duty 2.
void __stackcall Audio_Ch2NoteOn(u8 note, u8 vol_env, u8 duty2);
// Clamp the note index, program the raw NR32 output level, and trigger CH3.
// Wave RAM must already contain the desired waveform; this does not load one.
void __stackcall Audio_Ch3NoteOn(u8 note, u8 level);
// Cache the supplied noise parameter, program the raw envelope and polynomial
// registers, then trigger CH4 with the length-enable bit clear.
void __stackcall Audio_Ch4NoteOn(u8 param, u8 vol_env);

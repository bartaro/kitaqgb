#pragma bank 0

// Shared KITAQGB audio driver.
// Supports banked music streams, CH1/CH3 SFX, pan, sweep, CH3 wave changes,
// master volume control, and simple master-volume fades.
//
// GBDAW-Stream v1.2 additions:
// - Explicit per-channel automation commands for CH1 duty, CH2 duty,
//   CH3 level, CH4 noise parameter, and master volume.
// - Custom CH3 wave payload injection from the music stream.
//
// KITAQGB integration notes:
// - Keep this file in bank 0.
// - Compile your hardware-register file before this unit.
// - Audio_PlaySFX() now remembers the currently visible ROM bank so banked SFX
//   can keep playing correctly from Audio_Update().
// - Audio_PlaySFXBanked(bank, sfx, priority) is available when the caller wants
//   to specify the source bank explicitly.

#include "audio.h"

extern u8 __rom_bank;

#define AUDIO_RESUME_CH1 ((u8)0x01)
#define AUDIO_RESUME_CH2 ((u8)0x02)
#define AUDIO_RESUME_CH4 ((u8)0x04)
#define AUDIO_RESUME_CH3 ((u8)0x08)

// Global Variables (HRAM)
__hram u8 *Audio_MusicPointer;
__hram u8 *Audio_MusicLoopPoint;
__hram u8 Audio_MusicBank;
__hram u8 Audio_Delay;
__hram u8 Audio_PlayingMusic;
__hram u8 Audio_Paused;
__hram u8 aud_cmd;
__hram u8 aud_inst;
__hram u8 aud_note;

// Configuration State (WRAM)
__wram u8 Audio_MusicEnabled;
__wram u8 Audio_SfxEnabled;

// SFX Pointers
__hram u8 *Audio_EffectPointer;
__hram u8 Audio_EffectPriority;
__hram u8 Audio_EffectBank;
__hram u8 *Audio_EffectPointer3;
__hram u8 Audio_EffectPriority3;
__hram u8 Audio_EffectBank3;
__wram u8 Audio_EffectPan;
__wram u8 Audio_EffectPan3;
// The ordinary square-wave SFX stream normally owns CH1.  VBlank-backed
// games may move it to the equivalent CH2 pulse channel so CH1 lead notes can
// keep sounding without changing the SFX waveform or pitch.
__wram u8 Audio_EffectUsesCh2;

// Resume State
__hram u8 Audio_LastChMask;
__hram u8 Audio_LastCh1Note;
__hram u8 Audio_LastCh2Note;
__hram u8 Audio_LastCh4Param;
__wram u8 Audio_LastCh3Note;

// Instrument Parameters
__hram u8 Audio_Ch1Duty;
__hram u8 Audio_Ch2Duty;
__hram u8 Audio_Ch1Env;
__hram u8 Audio_Ch2Env;
__hram u8 Audio_Ch4Env;
__wram u8 Audio_Ch4Param;
__hram u8 Audio_Ch3Level;
__wram u8 Audio_Ch3Wave;

// Mix Parameters
__hram u8 Audio_Ch1Sweep;
__hram u8 Audio_Pan;
__wram u8 Audio_MasterLeft;
__wram u8 Audio_MasterRight;
__wram u8 Audio_FadeTargetLeft;
__wram u8 Audio_FadeTargetRight;
__wram u8 Audio_FadeStepFrames;
__wram u8 Audio_FadeCounter;
__wram u8 Audio_FadeActive;

// Custom CH3 wave cache
__wram u8 Audio_CustomWave[16];

__prg_rom u16 GB_FREQ_TABLE[] = {
    44,  156, 262, 363, 457, 547, 631, 710, 786, 854, 923, 986,
    1046,1102,1155,1205,1253,1297,1339,1379,1417,1452,1486,1517,
    1546,1575,1602,1627,1650,1673,1694,1714,1732,1750,1767,1783,
    1798,1812,1825,1837,1849,1860,1871,1881,1890,1899,1907,1915,
    1922,1929,1936,1942,1948,1954,1959,1964,1969,1973,1977,1981,
    /* C6..G6 fold to C5..G5 for a stable shared-driver upper range. */
    1922,1929,1936,1942,1948,1954,1959,1964
};

__prg_rom u8 AUDIO_NR50_LEFT_TABLE[] = {
    0x00, 0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70
};

__prg_rom u8 AUDIO_NR50_RIGHT_TABLE[] = {
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07
};

void Audio_UpdateNR51();
void Audio_RestoreCh3FromMusic();
void Audio_ReleaseCh1AfterSfx();
void __stackcall Audio_Ch2NoteOn(u8 note, u8 vol_env, u8 duty2);

#pragma fixed_bank 0
void __stackcall Audio_Ch3NoteOn(u8 note, u8 level) {
    u16 freq;
    u8 hi;

    if (note > AUDIO_NOTE_MAX) note = AUDIO_NOTE_MAX;
    freq = GB_FREQ_TABLE[note];

    NR30 = 128;
    NR31 = 0;
    NR32 = level;
    NR33 = (u8)(freq & 255);
    hi = (u8)((u16)freq >> 8);
    NR34 = (u8)(hi | 128);
}

void __stackcall Audio_Ch1NoteOn(u8 note, u8 vol_env, u8 duty2) {
    u16 freq;
    u8 duty_reg;
    u8 hi;

    if (note > AUDIO_NOTE_MAX) note = AUDIO_NOTE_MAX;
    freq = GB_FREQ_TABLE[note];

    duty_reg = 128;
    if (duty2 == 0) duty_reg = 0;
    else if (duty2 == 1) duty_reg = 64;
    else if (duty2 == 3) duty_reg = 192;

    NR10 = Audio_Ch1Sweep;
    NR11 = duty_reg;
    NR12 = vol_env;
    NR13 = (u8)(freq & 0xFF);
    hi = (u8)((u16)freq >> 8);
    NR14 = (u8)(hi | 0x80);
}

void Audio_ServiceSfx() {
    u8 __saved_bank;
    u8 n;
    u8 lev;
    u8 effNote;
    u8 effVol;
    u8 oldSweep;

    if (Audio_EffectPointer3 != 0) {
        __saved_bank = __rom_bank;
        if (__saved_bank != Audio_EffectBank3) __bankswitch(Audio_EffectBank3);

        n = *Audio_EffectPointer3;
        if (n == 0) {
            Audio_EffectPointer3 = 0;
            Audio_EffectPriority3 = 0;
            Audio_EffectBank3 = 0;
            Audio_EffectPan3 = AUDIO_PAN_INHERIT;
            if (__rom_bank != __saved_bank) __bankswitch(__saved_bank);
            Audio_UpdateNR51();
            Audio_RestoreCh3FromMusic();
        } else {
            Audio_EffectPointer3++;
            lev = *Audio_EffectPointer3;
            Audio_EffectPointer3++;

            if (__rom_bank != __saved_bank) __bankswitch(__saved_bank);
            if (Audio_SfxEnabled != 0) Audio_Ch3NoteOn(n, lev);
        }
    }

    if (Audio_EffectPointer != 0) {
        __saved_bank = __rom_bank;
        if (__saved_bank != Audio_EffectBank) __bankswitch(Audio_EffectBank);

        effNote = *Audio_EffectPointer;
        if (effNote == 0) {
            Audio_EffectPointer = 0;
            Audio_EffectPriority = 0;
            Audio_EffectBank = 0;
            Audio_EffectPan = AUDIO_PAN_INHERIT;
            if (__rom_bank != __saved_bank) __bankswitch(__saved_bank);
            Audio_UpdateNR51();
            Audio_ReleaseCh1AfterSfx();
        } else {
            Audio_EffectPointer++;
            effVol = *Audio_EffectPointer;
            Audio_EffectPointer++;

            if (__rom_bank != __saved_bank) __bankswitch(__saved_bank);
            if (Audio_SfxEnabled != 0) {
                if (Audio_EffectUsesCh2 != 0) {
                    Audio_Ch2NoteOn(effNote, effVol, 2);
                } else {
                    oldSweep = Audio_Ch1Sweep;
                    Audio_Ch1Sweep = 0;
                    Audio_Ch1NoteOn(effNote, effVol, 2);
                    Audio_Ch1Sweep = oldSweep;
                }
            }
        }
    }
}

void Audio_SetCustomWaveTriangle() {
    Audio_CustomWave[0] = 0x01; Audio_CustomWave[1] = 0x23;
    Audio_CustomWave[2] = 0x45; Audio_CustomWave[3] = 0x67;
    Audio_CustomWave[4] = 0x89; Audio_CustomWave[5] = 0xAB;
    Audio_CustomWave[6] = 0xCD; Audio_CustomWave[7] = 0xEF;
    Audio_CustomWave[8] = 0xFE; Audio_CustomWave[9] = 0xDC;
    Audio_CustomWave[10] = 0xBA; Audio_CustomWave[11] = 0x98;
    Audio_CustomWave[12] = 0x76; Audio_CustomWave[13] = 0x54;
    Audio_CustomWave[14] = 0x32; Audio_CustomWave[15] = 0x10;
}

void Audio_ApplyWaveBytes(const u8 *wave16) {
    NR30 = 0;
    WAVE0 = wave16[0];   WAVE1 = wave16[1];   WAVE2 = wave16[2];   WAVE3 = wave16[3];
    WAVE4 = wave16[4];   WAVE5 = wave16[5];   WAVE6 = wave16[6];   WAVE7 = wave16[7];
    WAVE8 = wave16[8];   WAVE9 = wave16[9];   WAVE10 = wave16[10]; WAVE11 = wave16[11];
    WAVE12 = wave16[12]; WAVE13 = wave16[13]; WAVE14 = wave16[14]; WAVE15 = wave16[15];
    NR30 = 128;
}

#ifndef AUDIO_EXCLUDE_WAVE_LOAD_API
void Audio_LoadWave0() {
    Audio_SetCustomWaveTriangle();
    Audio_ApplyWaveBytes(Audio_CustomWave);
}

void Audio_LoadWave1() {
    NR30 = 0;
    WAVE0  = 0x11; WAVE1  = 0x22; WAVE2  = 0x33; WAVE3  = 0x44;
    WAVE4  = 0x55; WAVE5  = 0x66; WAVE6  = 0x77; WAVE7  = 0x88;
    WAVE8  = 0x99; WAVE9  = 0xAA; WAVE10 = 0xBB; WAVE11 = 0xCC;
    WAVE12 = 0xDD; WAVE13 = 0xEE; WAVE14 = 0xFF; WAVE15 = 0x00;
    NR30 = 128;
}

void Audio_LoadWave2() {
    NR30 = 0;
    WAVE0 = 0x86; WAVE1 = 0x42; WAVE2 = 0x10; WAVE3 = 0x01;
    WAVE4 = 0x12; WAVE5 = 0x46; WAVE6 = 0x8A; WAVE7 = 0xCE;
    WAVE8 = 0xFF; WAVE9 = 0xCE; WAVE10 = 0x8A; WAVE11 = 0x46;
    WAVE12 = 0x12; WAVE13 = 0x01; WAVE14 = 0x10; WAVE15 = 0x42;
    NR30 = 128;
}

void Audio_LoadWave3() {
    NR30 = 0;
    WAVE0 = 0x01; WAVE1 = 0x23; WAVE2 = 0x45; WAVE3 = 0x67;
    WAVE4 = 0x89; WAVE5 = 0xAB; WAVE6 = 0xCD; WAVE7 = 0xEF;
    WAVE8 = 0xEE; WAVE9 = 0xDD; WAVE10 = 0xCC; WAVE11 = 0xBB;
    WAVE12 = 0xAA; WAVE13 = 0x99; WAVE14 = 0x88; WAVE15 = 0x77;
    NR30 = 128;
}

void Audio_LoadWave4() {
    NR30 = 0;
    WAVE0 = 0x78; WAVE1 = 0x9A; WAVE2 = 0xB9; WAVE3 = 0x87;
    WAVE4 = 0x65; WAVE5 = 0x43; WAVE6 = 0x21; WAVE7 = 0x10;
    WAVE8 = 0x01; WAVE9 = 0x12; WAVE10 = 0x34; WAVE11 = 0x56;
    WAVE12 = 0x78; WAVE13 = 0x9A; WAVE14 = 0xB9; WAVE15 = 0x87;
    NR30 = 128;
}

void Audio_LoadWave5() {
    NR30 = 0;
    WAVE0 = 0x8A; WAVE1 = 0xCF; WAVE2 = 0xD9; WAVE3 = 0x84;
    WAVE4 = 0x10; WAVE5 = 0x01; WAVE6 = 0x12; WAVE7 = 0x46;
    WAVE8 = 0x8A; WAVE9 = 0xCF; WAVE10 = 0xD9; WAVE11 = 0x84;
    WAVE12 = 0x10; WAVE13 = 0x01; WAVE14 = 0x12; WAVE15 = 0x46;
    NR30 = 128;
}

void Audio_LoadWave6() {
    NR30 = 0;
    WAVE0 = 0x45; WAVE1 = 0x67; WAVE2 = 0x89; WAVE3 = 0x98;
    WAVE4 = 0x76; WAVE5 = 0x54; WAVE6 = 0x32; WAVE7 = 0x10;
    WAVE8 = 0x01; WAVE9 = 0x23; WAVE10 = 0x45; WAVE11 = 0x67;
    WAVE12 = 0x89; WAVE13 = 0x98; WAVE14 = 0x76; WAVE15 = 0x54;
    NR30 = 128;
}
#endif

void Audio_SilenceCh1() {
    NR12 = 0;
    NR14 = 0x80;
}

void Audio_SilenceCh2() {
    NR22 = 0;
    NR24 = 0x80;
}

void Audio_Ch3Off() {
    NR32 = 0;
    NR30 = 0;
}

void Audio_SilenceCh4() {
    NR42 = 0;
    NR44 = 0x80;
}

void Audio_UpdateNR50() {
    NR50 = (u8)(AUDIO_NR50_LEFT_TABLE[Audio_MasterLeft] | AUDIO_NR50_RIGHT_TABLE[Audio_MasterRight]);
}

u8 __stackcall Audio_ApplyPanOverride(u8 mix, u8 ch, u8 pan) {
    u8 r_bit;
    u8 l_bit;
    u8 mask;

    if (ch == AUDIO_CHANNEL_CH1) {
        r_bit = 0x01;
        l_bit = 0x10;
    } else if (ch == AUDIO_CHANNEL_CH2) {
        r_bit = 0x02;
        l_bit = 0x20;
    } else if (ch == AUDIO_CHANNEL_CH3) {
        r_bit = 0x04;
        l_bit = 0x40;
    } else {
        r_bit = 0x08;
        l_bit = 0x80;
    }

    mask = (u8)(l_bit | r_bit);
    mix = (u8)(mix & (u8)~mask);

    if (pan == AUDIO_PAN_CENTER) mix = (u8)(mix | mask);
    else if (pan == AUDIO_PAN_LEFT) mix = (u8)(mix | l_bit);
    else if (pan == AUDIO_PAN_RIGHT) mix = (u8)(mix | r_bit);

    return mix;
}

void Audio_UpdateNR51() {
    u8 mix = Audio_Pan;
    if (Audio_EffectPointer != 0 && Audio_EffectPan != AUDIO_PAN_INHERIT) {
        if (Audio_EffectUsesCh2 != 0)
            mix = Audio_ApplyPanOverride(mix, AUDIO_CHANNEL_CH2, Audio_EffectPan);
        else
            mix = Audio_ApplyPanOverride(mix, AUDIO_CHANNEL_CH1, Audio_EffectPan);
    }
    if (Audio_EffectPointer3 != 0 && Audio_EffectPan3 != AUDIO_PAN_INHERIT) {
        mix = Audio_ApplyPanOverride(mix, AUDIO_CHANNEL_CH3, Audio_EffectPan3);
    }
    NR51 = mix;
}

void Audio_SilenceAll() {
    Audio_SilenceCh1();
    Audio_SilenceCh2();
    Audio_SilenceCh4();
    Audio_Ch3Off();
}

u8 Audio_ClampVolume(u8 value) {
    if (value > AUDIO_VOLUME_MAX) return AUDIO_VOLUME_MAX;
    return value;
}

#ifndef AUDIO_EXCLUDE_SFX_START_API
u8 Audio_SfxStartsOnCh3Banked(u8 bank, u8 *sfx) {
    u8 __saved_bank;
    u8 first;

    if (sfx == 0) return 0;

    __saved_bank = __rom_bank;
    if (__saved_bank != bank) __bankswitch(bank);
    first = *sfx;
    if (__rom_bank != __saved_bank) __bankswitch(__saved_bank);

    if (first == AUDIO_SFX_CH3_MARKER) return 1;
    return 0;
}
#endif

void Audio_RestoreCh1FromMusic() {
    if (Audio_PlayingMusic != 0 && Audio_MusicEnabled != 0 && (Audio_LastChMask & AUDIO_RESUME_CH1) != 0) {
        Audio_Ch1NoteOn(Audio_LastCh1Note, Audio_Ch1Env, Audio_Ch1Duty);
    } else {
        Audio_SilenceCh1();
    }
}

void Audio_ReleaseCh1AfterSfx() {
#ifdef AUDIO_VBLANK_SFX_RESTORE
    AudioVBlank_RequestRestoreCh1();
#else
    if (Audio_PlayingMusic == 0) Audio_SilenceCh1();
#endif
}

void Audio_RestoreCh3FromMusic() {
#ifdef AUDIO_VBLANK_SFX_RESTORE
    AudioVBlank_RequestRestoreCh3();
#else
    Audio_Ch3Off();
#endif
}

void __stackcall Audio_StopMusicChannelOnly(u8 ch) {
    if (ch == AUDIO_STREAM_CHANNEL_CH1) {
        Audio_LastChMask = (u8)(Audio_LastChMask & (u8)~AUDIO_RESUME_CH1);
        if (Audio_EffectPointer == 0) Audio_SilenceCh1();
    } else if (ch == AUDIO_STREAM_CHANNEL_CH2) {
        Audio_LastChMask = (u8)(Audio_LastChMask & (u8)~AUDIO_RESUME_CH2);
        Audio_SilenceCh2();
    } else if (ch == AUDIO_STREAM_CHANNEL_CH4) {
        Audio_LastChMask = (u8)(Audio_LastChMask & (u8)~AUDIO_RESUME_CH4);
        Audio_SilenceCh4();
    } else if (ch == AUDIO_STREAM_CHANNEL_CH3) {
        Audio_LastChMask = (u8)(Audio_LastChMask & (u8)~AUDIO_RESUME_CH3);
        if (Audio_EffectPointer3 == 0) Audio_Ch3Off();
    }
}

void Audio_UpdateFade() {
    if (Audio_FadeActive == 0) return;

    if (Audio_FadeCounter != 0) {
        Audio_FadeCounter--;
        if (Audio_FadeCounter != 0) return;
    }

    if (Audio_MasterLeft < Audio_FadeTargetLeft) Audio_MasterLeft++;
    else if (Audio_MasterLeft > Audio_FadeTargetLeft) Audio_MasterLeft--;

    if (Audio_MasterRight < Audio_FadeTargetRight) Audio_MasterRight++;
    else if (Audio_MasterRight > Audio_FadeTargetRight) Audio_MasterRight--;

    Audio_UpdateNR50();

    if (Audio_MasterLeft == Audio_FadeTargetLeft && Audio_MasterRight == Audio_FadeTargetRight) {
        Audio_FadeActive = 0;
        Audio_FadeCounter = 0;
    } else {
        Audio_FadeCounter = Audio_FadeStepFrames;
    }
}

void __stackcall Audio_SetMusicEnabled(u8 on) {
    if (on != 0) Audio_MusicEnabled = 1;
    else Audio_MusicEnabled = 0;
}

void __stackcall Audio_SetSfxEnabled(u8 on) {
    if (on != 0) {
        Audio_SfxEnabled = 1;
    } else {
        Audio_SfxEnabled = 0;
        Audio_StopSfx();
    }
}

void __stackcall Audio_SetMasterVolume(u8 left, u8 right) {
    Audio_MasterLeft = Audio_ClampVolume(left);
    Audio_MasterRight = Audio_ClampVolume(right);
    Audio_FadeTargetLeft = Audio_MasterLeft;
    Audio_FadeTargetRight = Audio_MasterRight;
    Audio_FadeStepFrames = 0;
    Audio_FadeCounter = 0;
    Audio_FadeActive = 0;
    Audio_UpdateNR50();
}

void __stackcall Audio_FadeToMasterVolume(u8 left, u8 right, u8 step_frames) {
    Audio_FadeTargetLeft = Audio_ClampVolume(left);
    Audio_FadeTargetRight = Audio_ClampVolume(right);

    if (step_frames == 0) {
        Audio_SetMasterVolume(Audio_FadeTargetLeft, Audio_FadeTargetRight);
        return;
    }

    Audio_FadeStepFrames = step_frames;
    Audio_FadeCounter = step_frames;
    if (Audio_MasterLeft == Audio_FadeTargetLeft && Audio_MasterRight == Audio_FadeTargetRight) {
        Audio_FadeActive = 0;
        Audio_FadeCounter = 0;
    } else {
        Audio_FadeActive = 1;
    }
}

void Audio_CancelMasterVolumeFade() {
    Audio_FadeTargetLeft = Audio_MasterLeft;
    Audio_FadeTargetRight = Audio_MasterRight;
    Audio_FadeStepFrames = 0;
    Audio_FadeCounter = 0;
    Audio_FadeActive = 0;
}

void __stackcall Audio_SetPan(u8 ch, u8 pan) {
    u8 r_bit;
    u8 l_bit;
    u8 mask;

    if (ch == AUDIO_CHANNEL_CH1) {
        r_bit = 0x01;
        l_bit = 0x10;
    } else if (ch == AUDIO_CHANNEL_CH2) {
        r_bit = 0x02;
        l_bit = 0x20;
    } else if (ch == AUDIO_CHANNEL_CH3) {
        r_bit = 0x04;
        l_bit = 0x40;
    } else {
        r_bit = 0x08;
        l_bit = 0x80;
    }

    mask = (u8)(l_bit | r_bit);
    Audio_Pan = (u8)(Audio_Pan & (u8)~mask);

    if (pan == AUDIO_PAN_CENTER) Audio_Pan = (u8)(Audio_Pan | mask);
    else if (pan == AUDIO_PAN_LEFT) Audio_Pan = (u8)(Audio_Pan | l_bit);
    else if (pan == AUDIO_PAN_RIGHT) Audio_Pan = (u8)(Audio_Pan | r_bit);

    Audio_UpdateNR51();
}

void __stackcall Audio_SetPanPacked(u8 packed) {
    u8 mix = 0;
    u8 pan = (u8)(packed & 3);
    if (pan == AUDIO_PAN_CENTER) mix = (u8)(mix | 0x11);
    else if (pan == AUDIO_PAN_LEFT) mix = (u8)(mix | 0x10);
    else if (pan == AUDIO_PAN_RIGHT) mix = (u8)(mix | 0x01);
    pan = (u8)((packed >> 2) & 3);
    if (pan == AUDIO_PAN_CENTER) mix = (u8)(mix | 0x22);
    else if (pan == AUDIO_PAN_LEFT) mix = (u8)(mix | 0x20);
    else if (pan == AUDIO_PAN_RIGHT) mix = (u8)(mix | 0x02);
    pan = (u8)((packed >> 4) & 3);
    if (pan == AUDIO_PAN_CENTER) mix = (u8)(mix | 0x44);
    else if (pan == AUDIO_PAN_LEFT) mix = (u8)(mix | 0x40);
    else if (pan == AUDIO_PAN_RIGHT) mix = (u8)(mix | 0x04);
    pan = (u8)((packed >> 6) & 3);
    if (pan == AUDIO_PAN_CENTER) mix = (u8)(mix | 0x88);
    else if (pan == AUDIO_PAN_LEFT) mix = (u8)(mix | 0x80);
    else if (pan == AUDIO_PAN_RIGHT) mix = (u8)(mix | 0x08);
    Audio_Pan = mix;
    Audio_UpdateNR51();
}

#ifndef AUDIO_EXCLUDE_WAVE_LOAD_API
void __stackcall Audio_LoadCustomWave(const u8 *wave16) {
    if (wave16 == 0) return;

    Audio_CustomWave[0] = wave16[0];   Audio_CustomWave[1] = wave16[1];
    Audio_CustomWave[2] = wave16[2];   Audio_CustomWave[3] = wave16[3];
    Audio_CustomWave[4] = wave16[4];   Audio_CustomWave[5] = wave16[5];
    Audio_CustomWave[6] = wave16[6];   Audio_CustomWave[7] = wave16[7];
    Audio_CustomWave[8] = wave16[8];   Audio_CustomWave[9] = wave16[9];
    Audio_CustomWave[10] = wave16[10]; Audio_CustomWave[11] = wave16[11];
    Audio_CustomWave[12] = wave16[12]; Audio_CustomWave[13] = wave16[13];
    Audio_CustomWave[14] = wave16[14]; Audio_CustomWave[15] = wave16[15];

    Audio_ApplyWaveBytes(Audio_CustomWave);
}

void __stackcall Audio_LoadWave(u8 wave_id) {
    if (wave_id == AUDIO_WAVE_TRIANGLE) Audio_LoadWave0();
    else if (wave_id == AUDIO_WAVE_SAW) Audio_LoadWave1();
    else if (wave_id == AUDIO_WAVE_VOWEL_A) Audio_LoadWave2();
    else if (wave_id == AUDIO_WAVE_VOWEL_I) Audio_LoadWave3();
    else if (wave_id == AUDIO_WAVE_VOWEL_U) Audio_LoadWave4();
    else if (wave_id == AUDIO_WAVE_VOWEL_E) Audio_LoadWave5();
    else if (wave_id == AUDIO_WAVE_VOWEL_O) Audio_LoadWave6();
    else if (wave_id == AUDIO_WAVE_CUSTOM) Audio_ApplyWaveBytes(Audio_CustomWave);
}
#endif

void __stackcall Audio_SetCh1Duty(u8 duty) {
    Audio_Ch1Duty = (u8)(duty & 3);
}

void __stackcall Audio_SetCh2Duty(u8 duty) {
    Audio_Ch2Duty = (u8)(duty & 3);
}

void __stackcall Audio_SetCh3Level(u8 level) {
    Audio_Ch3Level = level;
}

void __stackcall Audio_SetCh4Param(u8 param) {
    Audio_Ch4Param = param;
}

#ifndef AUDIO_EXCLUDE_CH2_CH4_NOTE_API
void __stackcall Audio_Ch2NoteOn(u8 note, u8 vol_env, u8 duty2) {
    u16 freq;
    u8 duty_reg;
    u8 hi;

    if (note > AUDIO_NOTE_MAX) note = AUDIO_NOTE_MAX;
    freq = GB_FREQ_TABLE[note];

    duty_reg = 128;
    if (duty2 == 0) duty_reg = 0;
    else if (duty2 == 1) duty_reg = 64;
    else if (duty2 == 3) duty_reg = 192;

    NR21 = duty_reg;
    NR22 = vol_env;
    NR23 = (u8)(freq & 0xFF);
    hi = (u8)((u16)freq >> 8);
    NR24 = (u8)(hi | 0x80);
}

void __stackcall Audio_Ch4NoteOn(u8 param, u8 vol_env) {
    Audio_SetCh4Param(param);
    NR41 = 0x20;
    NR42 = vol_env;
    NR43 = Audio_Ch4Param;
    NR44 = 0x80;
}
#endif

#ifndef AUDIO_EXCLUDE_LEGACY_MUSIC_SERVICE
void Audio_ServiceMusic() {
    u8 __saved_bank;
    u8 inst;
    u8 duty;
    u8 env;
    u8 ch;
    u8 pan;
    u8 val;
    u8 wave_id;
    u8 left;
    u8 right;

    if (Audio_PlayingMusic == 0) return;

    __saved_bank = __rom_bank;
    if (__saved_bank != Audio_MusicBank) __bankswitch(Audio_MusicBank);

    if (Audio_Delay > 0) {
        Audio_Delay--;
    } else {
        while (1) {
            aud_cmd = *Audio_MusicPointer;
            Audio_MusicPointer++;

            if (aud_cmd == AUDIO_CMD_WAIT) {
                Audio_Delay = *Audio_MusicPointer;
                Audio_MusicPointer++;
                break;
            } else if (aud_cmd == AUDIO_CMD_NOTE) {
                aud_inst = *Audio_MusicPointer;
                Audio_MusicPointer++;
                aud_note = *Audio_MusicPointer;
                Audio_MusicPointer++;

                if (Audio_MusicEnabled != 0) {
                    if (aud_inst == AUDIO_STREAM_CHANNEL_CH1) {
                        if (Audio_EffectPointer == 0) Audio_Ch1NoteOn(aud_note, Audio_Ch1Env, Audio_Ch1Duty);
                        Audio_LastCh1Note = aud_note;
                        Audio_LastChMask = (u8)(Audio_LastChMask | AUDIO_RESUME_CH1);
                    } else if (aud_inst == AUDIO_STREAM_CHANNEL_CH2) {
                        Audio_Ch2NoteOn(aud_note, Audio_Ch2Env, Audio_Ch2Duty);
                        Audio_LastCh2Note = aud_note;
                        Audio_LastChMask = (u8)(Audio_LastChMask | AUDIO_RESUME_CH2);
                    } else if (aud_inst == AUDIO_STREAM_CHANNEL_CH4) {
                        Audio_Ch4NoteOn(aud_note, Audio_Ch4Env);
                        Audio_LastCh4Param = Audio_Ch4Param;
                        Audio_LastChMask = (u8)(Audio_LastChMask | AUDIO_RESUME_CH4);
                    } else if (aud_inst == AUDIO_STREAM_CHANNEL_CH3) {
                        if (Audio_EffectPointer3 == 0) Audio_Ch3NoteOn(aud_note, Audio_Ch3Level);
                        Audio_LastCh3Note = aud_note;
                        Audio_LastChMask = (u8)(Audio_LastChMask | AUDIO_RESUME_CH3);
                    }
                }
            } else if (aud_cmd == AUDIO_CMD_SET_INST) {
                inst = *Audio_MusicPointer;
                Audio_MusicPointer++;
                duty = *Audio_MusicPointer;
                Audio_MusicPointer++;
                env = *Audio_MusicPointer;
                Audio_MusicPointer++;

                if (inst == AUDIO_STREAM_CHANNEL_CH1) {
                    Audio_SetCh1Duty(duty);
                    Audio_Ch1Env = env;
                } else if (inst == AUDIO_STREAM_CHANNEL_CH2) {
                    Audio_SetCh2Duty(duty);
                    Audio_Ch2Env = env;
                } else if (inst == AUDIO_STREAM_CHANNEL_CH4) {
                    Audio_Ch4Env = env;
                } else if (inst == AUDIO_STREAM_CHANNEL_CH3) {
                    Audio_SetCh3Level(env);
                }
            } else if (aud_cmd == AUDIO_CMD_SET_PAN) {
                ch = *Audio_MusicPointer;
                Audio_MusicPointer++;
                pan = *Audio_MusicPointer;
                Audio_MusicPointer++;
                Audio_SetPan(ch, pan);
            } else if (aud_cmd == AUDIO_CMD_SET_SWEEP) {
                val = *Audio_MusicPointer;
                Audio_MusicPointer++;
                Audio_Ch1Sweep = val;
            } else if (aud_cmd == AUDIO_CMD_SET_WAVE) {
                wave_id = *Audio_MusicPointer;
                Audio_MusicPointer++;
                Audio_Ch3Wave = wave_id;
                Audio_LoadWave(wave_id);
            } else if (aud_cmd == AUDIO_CMD_SET_CH3_CUSTOM_WAVE) {
                Audio_Ch3Wave = AUDIO_WAVE_CUSTOM;
                Audio_LoadCustomWave(Audio_MusicPointer);
                Audio_MusicPointer += 16;
            } else if (aud_cmd == AUDIO_CMD_SET_CH4_PARAM) {
                Audio_SetCh4Param(*Audio_MusicPointer);
                Audio_MusicPointer++;
            } else if (aud_cmd == AUDIO_CMD_SET_MASTER_VOLUME) {
                left = *Audio_MusicPointer;
                Audio_MusicPointer++;
                right = *Audio_MusicPointer;
                Audio_MusicPointer++;
                Audio_SetMasterVolume(left, right);
            } else if (aud_cmd == AUDIO_CMD_SET_CH1_DUTY) {
                Audio_SetCh1Duty(*Audio_MusicPointer);
                Audio_MusicPointer++;
            } else if (aud_cmd == AUDIO_CMD_SET_CH2_DUTY) {
                Audio_SetCh2Duty(*Audio_MusicPointer);
                Audio_MusicPointer++;
            } else if (aud_cmd == AUDIO_CMD_SET_CH3_LEVEL) {
                Audio_SetCh3Level(*Audio_MusicPointer);
                Audio_MusicPointer++;
            } else if (aud_cmd == AUDIO_CMD_STOP_CHANNEL) {
                ch = *Audio_MusicPointer;
                Audio_MusicPointer++;
                if (Audio_MusicEnabled != 0) Audio_StopMusicChannelOnly(ch);
                else {
                    if (ch == AUDIO_STREAM_CHANNEL_CH1) Audio_LastChMask = (u8)(Audio_LastChMask & (u8)~AUDIO_RESUME_CH1);
                    else if (ch == AUDIO_STREAM_CHANNEL_CH2) Audio_LastChMask = (u8)(Audio_LastChMask & (u8)~AUDIO_RESUME_CH2);
                    else if (ch == AUDIO_STREAM_CHANNEL_CH4) Audio_LastChMask = (u8)(Audio_LastChMask & (u8)~AUDIO_RESUME_CH4);
                    else if (ch == AUDIO_STREAM_CHANNEL_CH3) Audio_LastChMask = (u8)(Audio_LastChMask & (u8)~AUDIO_RESUME_CH3);
                }
            } else if (aud_cmd == AUDIO_CMD_LOOP) {
                Audio_MusicPointer = Audio_MusicLoopPoint;
            } else if (aud_cmd == AUDIO_CMD_STOP) {
                Audio_PlayingMusic = 0;
                Audio_LastChMask = 0;
                Audio_SilenceAll();
                break;
            } else {
                // Defensive fail-closed behavior for unknown stream commands.
                // Silently desynchronizing the stream corrupts all later audio.
                Audio_PlayingMusic = 0;
                Audio_LastChMask = 0;
                Audio_SilenceAll();
                break;
            }
        }
    }

    if (__rom_bank != __saved_bank) __bankswitch(__saved_bank);
}
#endif

#pragma fixed_bank 1
void Audio_Update() {
    Audio_UpdateFade();
    if (Audio_Paused != 0) return;

    Audio_ServiceSfx();
#ifndef AUDIO_EXCLUDE_LEGACY_MUSIC_SERVICE
    Audio_ServiceMusic();
#endif
}
#pragma fixed_bank -1

#pragma fixed_bank 1
void Audio_Init() {
    NR52 = 0x80;
    NR51 = 0xFF;
    Audio_Pan = 0xFF;
    Audio_EffectPan = AUDIO_PAN_INHERIT;
    Audio_EffectPan3 = AUDIO_PAN_INHERIT;
    Audio_UpdateNR51();

    Audio_SetMasterVolume(AUDIO_VOLUME_MAX, AUDIO_VOLUME_MAX);
    Audio_SilenceAll();

    Audio_PlayingMusic = 0;
    Audio_Paused = 0;
    Audio_MusicEnabled = 1;
    Audio_SfxEnabled = 1;
    Audio_Delay = 0;
    Audio_MusicBank = 0;
    Audio_MusicPointer = 0;
    Audio_MusicLoopPoint = 0;

    Audio_EffectPointer = 0;
    Audio_EffectPriority = 0;
    Audio_EffectBank = 0;
    Audio_EffectPointer3 = 0;
    Audio_EffectPriority3 = 0;
    Audio_EffectBank3 = 0;
    Audio_EffectUsesCh2 = 0;
    Audio_EffectPan = AUDIO_PAN_INHERIT;
    Audio_EffectPan3 = AUDIO_PAN_INHERIT;

    Audio_LastChMask = 0;
    Audio_LastCh1Note = 0;
    Audio_LastCh2Note = 0;
    Audio_LastCh3Note = 0;
    Audio_LastCh4Param = 0;

    Audio_Ch1Duty = 2;
    Audio_Ch2Duty = 2;
    Audio_Ch1Env = 0xF1;
    Audio_Ch2Env = 0xF2;
    Audio_Ch4Env = 0xF1;
    Audio_Ch4Param = 0x00;
    Audio_Ch3Level = AUDIO_CH3_LEVEL_100;
    Audio_Ch3Wave = AUDIO_WAVE_TRIANGLE;
    Audio_Ch1Sweep = 0x00;

    Audio_SetCustomWaveTriangle();
#ifdef AUDIO_EXCLUDE_WAVE_LOAD_API
    Audio_ApplyWaveBytes(Audio_CustomWave);
#else
    Audio_LoadWave(Audio_Ch3Wave);
#endif
    Audio_Ch3Off();
}
#pragma fixed_bank -1

#ifndef AUDIO_EXCLUDE_MUSIC_START_API
void __stackcall Audio_PlayMusic(u8 bank, u8 *song) {
    Audio_PlayingMusic = 0;
    Audio_MusicBank = bank;
    Audio_MusicPointer = song;
    Audio_MusicLoopPoint = song;
    Audio_Delay = 0;
    Audio_LastChMask = 0;

    Audio_Ch1Duty = 2;
    Audio_Ch2Duty = 2;
    Audio_Ch1Env = 0xF1;
    Audio_Ch2Env = 0xF2;
    Audio_Ch4Env = 0xF1;
    Audio_Ch4Param = 0x00;
    Audio_Ch3Level = AUDIO_CH3_LEVEL_100;
    Audio_Ch3Wave = AUDIO_WAVE_TRIANGLE;
    Audio_Ch1Sweep = 0x00;
    Audio_Pan = 0xFF;
    Audio_UpdateNR51();
    Audio_LoadWave(Audio_Ch3Wave);

    Audio_PlayingMusic = 1;
}
#endif

#pragma fixed_bank 1
void Audio_StopMusic() {
    Audio_PlayingMusic = 0;
    Audio_LastChMask = 0;
    Audio_SilenceAll();
}

void Audio_StopSfx() {
    Audio_EffectPointer = 0;
    Audio_EffectPriority = 0;
    Audio_EffectBank = 0;
    Audio_EffectPointer3 = 0;
    Audio_EffectPriority3 = 0;
    Audio_EffectBank3 = 0;
    Audio_EffectPan = AUDIO_PAN_INHERIT;
    Audio_EffectPan3 = AUDIO_PAN_INHERIT;
    Audio_UpdateNR51();

    if (Audio_Paused != 0) return;
    Audio_ReleaseCh1AfterSfx();
    Audio_RestoreCh3FromMusic();
}
#pragma fixed_bank -1

#ifndef AUDIO_EXCLUDE_CHANNEL_PAUSE_API
void __stackcall Audio_StopChannel(u8 ch) {
    if (ch == AUDIO_CHANNEL_CH1) {
        Audio_EffectPointer = 0;
        Audio_EffectPriority = 0;
        Audio_EffectBank = 0;
        Audio_EffectPan = AUDIO_PAN_INHERIT;
        Audio_LastChMask = (u8)(Audio_LastChMask & (u8)~AUDIO_RESUME_CH1);
        Audio_UpdateNR51();
        Audio_SilenceCh1();
    } else if (ch == AUDIO_CHANNEL_CH2) {
        Audio_LastChMask = (u8)(Audio_LastChMask & (u8)~AUDIO_RESUME_CH2);
        Audio_SilenceCh2();
    } else if (ch == AUDIO_CHANNEL_CH3) {
        Audio_EffectPointer3 = 0;
        Audio_EffectPriority3 = 0;
        Audio_EffectBank3 = 0;
        Audio_EffectPan3 = AUDIO_PAN_INHERIT;
        Audio_LastChMask = (u8)(Audio_LastChMask & (u8)~AUDIO_RESUME_CH3);
        Audio_UpdateNR51();
        Audio_Ch3Off();
    } else if (ch == AUDIO_CHANNEL_CH4) {
        Audio_LastChMask = (u8)(Audio_LastChMask & (u8)~AUDIO_RESUME_CH4);
        Audio_SilenceCh4();
    }
}

void __stackcall Audio_SetPaused(u8 on) {
    if (on != 0) {
        if (Audio_Paused == 0) {
            Audio_Paused = 1;
            Audio_SilenceAll();
        }
    } else {
        if (Audio_Paused != 0) {
            Audio_Paused = 0;
            if (Audio_PlayingMusic != 0 && Audio_MusicEnabled != 0) {
                if ((Audio_LastChMask & AUDIO_RESUME_CH1) != 0 && Audio_EffectPointer == 0) {
                    Audio_Ch1NoteOn(Audio_LastCh1Note, Audio_Ch1Env, Audio_Ch1Duty);
                }
                if ((Audio_LastChMask & AUDIO_RESUME_CH2) != 0) {
                    Audio_Ch2NoteOn(Audio_LastCh2Note, Audio_Ch2Env, Audio_Ch2Duty);
                }
                if ((Audio_LastChMask & AUDIO_RESUME_CH4) != 0) {
                    Audio_Ch4NoteOn(Audio_LastCh4Param, Audio_Ch4Env);
                }
            }
        }
    }
}
#endif

#ifndef AUDIO_EXCLUDE_SFX_START_API
#pragma fixed_bank 0
void __stackcall Audio_PlaySFXPannedBanked(u8 bank, u8 *sfx, u8 priority, u8 pan) {
    u8 sfx_pan = pan;
    u8 use_ch3 = 0;
    if (sfx_pan > AUDIO_PAN_MUTE) sfx_pan = AUDIO_PAN_INHERIT;

    if (Audio_Paused != 0) return;
    if (sfx != 0) {
        use_ch3 = Audio_SfxStartsOnCh3Banked(bank, sfx);
        if (use_ch3 != 0) {
            if (Audio_EffectPointer3 == 0 || priority >= Audio_EffectPriority3) {
                Audio_EffectPointer3 = sfx + 1;
                Audio_EffectPriority3 = priority;
                Audio_EffectBank3 = bank;
                Audio_EffectPan3 = sfx_pan;
                Audio_UpdateNR51();
            }
            return;
        }
    }
    if (Audio_EffectPointer == 0 || priority >= Audio_EffectPriority) {
#ifdef AUDIO_VBLANK_SFX_AUTO_CH2
        Audio_EffectUsesCh2 = (u8)(AudioVBlank_MusicPlaying != 0 &&
                                   AudioVBlank_MusicEnabled != 0);
#else
        Audio_EffectUsesCh2 = 0;
#endif
        Audio_EffectPointer = sfx;
        Audio_EffectPriority = priority;
        Audio_EffectBank = bank;
        Audio_EffectPan = sfx_pan;
        Audio_UpdateNR51();
    }
}

void __stackcall Audio_PlaySFXBanked(u8 bank, u8 *sfx, u8 priority) {
    Audio_PlaySFXPannedBanked(bank, sfx, priority, AUDIO_PAN_INHERIT);
}

#ifndef AUDIO_EXCLUDE_IMPLICIT_BANK_SFX
void __stackcall Audio_PlaySFX(u8 *sfx, u8 priority) {
    Audio_PlaySFXBanked(__rom_bank, sfx, priority);
}

void __stackcall Audio_PlaySFXPanned(u8 *sfx, u8 priority, u8 pan) {
    Audio_PlaySFXPannedBanked(__rom_bank, sfx, priority, pan);
}

#endif

#pragma fixed_bank -1
#endif

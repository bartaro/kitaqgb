/* Copyright (c) 2026 DAISUKE OBA. SPDX-License-Identifier: MIT.
   Resident VBlank song records: delay, CH1 pulse, CH2 pulse, CH3 wave, CH4 noise. Delays are
   interrupt ticks; channel sentinels are described below. */

#pragma bank 0

// Snake Game BGM - "Min'yo Jazz Techno (White Snake)" [40 Bars, Full Version]
// Faithful VBlank-safe transcription of the provided standard audio.c stream.
// Source style: Japanese folk x jazz x techno, BPM 108 swing.
//
// VBlank driver record format:
//   delay, ch1_melody_note, ch2_stab_note, ch3_bass_note, ch4_noise_param

#include "rpg.h"
#include "audio_vblank.h"

/* REST keeps a channel's current sound; it is not a note-off command. */
#define N  AUDIO_VBLANK_REST
#define SND_VB_LOOP AUDIO_VBLANK_LOOP
#define SND_VB_END AUDIO_VBLANK_END

#define SND_VB_WL ((u8)22)
#define SND_VB_WS ((u8)11)

#define DK ((u8)0x43)
#define DS ((u8)0x51)
#define DH ((u8)0x21)

#define R(d,c1,c2,c3,c4) (d), (c1), (c2), (c3), (c4)

__prg_rom u8 SND_VB_GAMESTART_BGM[] = {
    /* Fast shamisen-style climb */
    R(11, 36, 36, 24, DK),
    R(11, 39, N, N, DH),
    R(11, 41, 39, 31, DS),
    R(11, 43, N, N, DH),
    R(11, 46, 41, 34, DK),
    R(11, 48, N, N, DH),
    R(11, 51, 43, 36, DS),
    R(11, 51, N, N, DH),

    /* Final hit */
    R(88, 55, 48, 43, DK),

    SND_VB_END
};

__prg_rom u8 SND_VB_BGM[] = {
    /* Bar 1 */
    R(SND_VB_WL, N, N, 24, DK), R(SND_VB_WS, N, 36, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, N, 39, N, DH),
    R(SND_VB_WL, N, N, 34, DK), R(SND_VB_WS, N, 36, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, N, 34, N, DH),

    /* Bar 2 */
    R(SND_VB_WL, N, N, 24, DK), R(SND_VB_WS, N, 36, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, N, 39, N, DH),
    R(SND_VB_WL, N, N, 34, DK), R(SND_VB_WS, N, 36, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, N, 34, N, DH),

    /* Bar 3 */
    R(SND_VB_WL, N, N, 24, DK), R(SND_VB_WS, N, 36, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, N, 39, N, DH),
    R(SND_VB_WL, N, N, 34, DK), R(SND_VB_WS, N, 36, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, N, 34, N, DH),

    /* Bar 4 */
    R(SND_VB_WL, N, N, 24, DK), R(SND_VB_WS, N, 36, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, N, 39, N, DH),
    R(SND_VB_WL, N, N, 34, DK), R(SND_VB_WS, N, 36, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, N, 34, N, DH),

    /* Bar 5 */
    R(SND_VB_WL, 36, N, 24, DK), R(SND_VB_WS, N, 36, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, 36, 39, N, DH),
    R(SND_VB_WL, 39, N, 34, DK), R(SND_VB_WS, N, 36, N, DH), R(SND_VB_WL, 41, N, 31, DS), R(SND_VB_WS, N, 34, N, DH),

    /* Bar 6 */
    R(SND_VB_WL, 43, N, 24, DK), R(SND_VB_WS, N, 36, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, 41, 39, N, DH),
    R(SND_VB_WL, 39, N, 34, DK), R(SND_VB_WS, N, 36, N, DH), R(SND_VB_WL, 36, N, 31, DS), R(SND_VB_WS, N, 34, N, DH),

    /* Bar 7 */
    R(SND_VB_WL, 34, N, 24, DK), R(SND_VB_WS, 36, 36, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, N, 39, N, DH),
    R(SND_VB_WL, 39, N, 34, DK), R(SND_VB_WS, 41, 36, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, N, 34, N, DH),

    /* Bar 8 */
    R(SND_VB_WL, 43, N, 24, DK), R(SND_VB_WS, N, 36, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, N, 39, N, DH),
    R(SND_VB_WL, 46, N, 34, DK), R(SND_VB_WS, N, 36, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, N, 34, N, DH),

    /* Bar 9 */
    R(SND_VB_WL, 41, N, 17, DK), R(SND_VB_WS, N, 36, N, DH), R(SND_VB_WL, N, N, 24, DS), R(SND_VB_WS, 43, 39, N, DH),
    R(SND_VB_WL, 46, N, 27, DK), R(SND_VB_WS, N, 36, N, DH), R(SND_VB_WL, 46, N, 24, DS), R(SND_VB_WS, N, 34, N, DH),

    /* Bar 10 */
    R(SND_VB_WL, 46, N, 17, DK), R(SND_VB_WS, N, 36, N, DH), R(SND_VB_WL, N, N, 24, DS), R(SND_VB_WS, 46, 39, N, DH),
    R(SND_VB_WL, 43, N, 27, DK), R(SND_VB_WS, N, 36, N, DH), R(SND_VB_WL, 41, N, 24, DS), R(SND_VB_WS, N, 34, N, DH),

    /* Bar 11 */
    R(SND_VB_WL, 39, N, 24, DK), R(SND_VB_WS, 41, 36, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, N, 39, N, DH),
    R(SND_VB_WL, 43, N, 34, DK), R(SND_VB_WS, 46, 36, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, N, 34, N, DH),

    /* Bar 12 */
    R(SND_VB_WL, 46, N, 24, DK), R(SND_VB_WS, N, 36, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, N, 39, N, DH),
    R(SND_VB_WL, 36, N, 34, DK), R(SND_VB_WS, N, 36, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, N, 34, N, DH),

    /* Bar 13 */
    R(SND_VB_WL, 44, N, 20, DK), R(SND_VB_WS, N, 44, N, DH), R(SND_VB_WL, N, N, 32, DS), R(SND_VB_WS, 44, 39, N, DH),
    R(SND_VB_WL, 39, N, 27, DK), R(SND_VB_WS, N, 36, N, DH), R(SND_VB_WL, N, N, 24, DS), R(SND_VB_WS, N, 34, N, DH),

    /* Bar 14 */
    R(SND_VB_WL, 43, N, 19, DK), R(SND_VB_WS, N, 43, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, 43, 38, N, DH),
    R(SND_VB_WL, 38, N, 26, DK), R(SND_VB_WS, N, 34, N, DH), R(SND_VB_WL, N, N, 23, DS), R(SND_VB_WS, N, 31, N, DH),

    /* Bar 15 */
    R(SND_VB_WL, 41, N, 17, DK), R(SND_VB_WS, N, 41, N, DH), R(SND_VB_WL, N, N, 29, DS), R(SND_VB_WS, 39, 36, N, DH),
    R(SND_VB_WL, 36, N, 24, DK), R(SND_VB_WS, N, 34, N, DH), R(SND_VB_WL, 34, N, 20, DS), R(SND_VB_WS, N, 29, N, DH),

    /* Bar 16 */
    R(SND_VB_WL, 43, N, 19, DK), R(SND_VB_WS, N, 46, N, DH), R(SND_VB_WL, 46, N, 31, DS), R(SND_VB_WS, N, 43, N, DH),
    R(SND_VB_WL, 46, N, 19, DK), R(SND_VB_WS, 46, 46, N, DH), R(SND_VB_WL, 43, N, 31, DS), R(SND_VB_WS, N, 43, N, DH),

    /* Bar 17 */
    R(SND_VB_WL, 44, N, 20, DK), R(SND_VB_WS, N, 44, N, DH), R(SND_VB_WL, N, N, 32, DS), R(SND_VB_WS, 44, 39, N, DH),
    R(SND_VB_WL, 46, N, 27, DK), R(SND_VB_WS, N, 36, N, DH), R(SND_VB_WL, N, N, 24, DS), R(SND_VB_WS, N, 34, N, DH),

    /* Bar 18 */
    R(SND_VB_WL, 46, N, 19, DK), R(SND_VB_WS, N, 43, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, 46, 38, N, DH),
    R(SND_VB_WL, 43, N, 26, DK), R(SND_VB_WS, N, 34, N, DH), R(SND_VB_WL, N, N, 23, DS), R(SND_VB_WS, N, 31, N, DH),

    /* Bar 19 */
    R(SND_VB_WL, 41, N, 17, DK), R(SND_VB_WS, N, 41, N, DH), R(SND_VB_WL, N, N, 29, DS), R(SND_VB_WS, 39, 36, N, DH),
    R(SND_VB_WL, 36, N, 24, DK), R(SND_VB_WS, N, 34, N, DH), R(SND_VB_WL, 34, N, 20, DS), R(SND_VB_WS, N, 29, N, DH),

    /* Bar 20 */
    R(SND_VB_WL, 43, N, 19, DK), R(SND_VB_WS, N, 46, N, DH), R(SND_VB_WL, 46, N, 31, DS), R(SND_VB_WS, N, N, N, DS),
    R(SND_VB_WS, 46, N, 19, DS), R(SND_VB_WS, N, N, N, DS), R(SND_VB_WS, 46, 46, N, DH), R(SND_VB_WL, 43, N, 31, DS),
    R(SND_VB_WS, N, 43, N, DH),

    /* Bar 21 */
    R(SND_VB_WL, 36, N, 12, DK), R(SND_VB_WS, N, 36, N, DH), R(SND_VB_WL, N, N, 24, DS), R(SND_VB_WS, 39, 39, N, DH),
    R(SND_VB_WL, 43, N, 12, DK), R(SND_VB_WS, N, 36, N, DH), R(SND_VB_WL, N, N, 24, DS), R(SND_VB_WS, 46, 34, N, DH),

    /* Bar 22 */
    R(SND_VB_WL, 46, N, 15, DK), R(SND_VB_WS, N, 39, N, DH), R(SND_VB_WL, N, N, 27, DS), R(SND_VB_WS, 46, 43, N, DH),
    R(SND_VB_WL, 43, N, 15, DK), R(SND_VB_WS, N, 39, N, DH), R(SND_VB_WL, N, N, 27, DS), R(SND_VB_WS, 39, 36, N, DH),

    /* Bar 23 */
    R(SND_VB_WL, 41, N, 17, DK), R(SND_VB_WS, N, 41, N, DH), R(SND_VB_WL, N, N, 29, DS), R(SND_VB_WS, 39, 44, N, DH),
    R(SND_VB_WL, 36, N, 17, DK), R(SND_VB_WS, N, 41, N, DH), R(SND_VB_WL, N, N, 29, DS), R(SND_VB_WS, 34, 39, N, DH),

    /* Bar 24 */
    R(SND_VB_WL, 43, N, 19, DK), R(SND_VB_WS, N, 43, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, 46, 46, N, DH),
    R(SND_VB_WL, 46, N, 19, DK), R(SND_VB_WS, 43, 43, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, 46, 46, N, DH),

    /* Bars 25-28 */
    R(SND_VB_WL, 36, N, 12, DK), R(SND_VB_WS, N, 36, N, DH), R(SND_VB_WL, N, N, 24, DS), R(SND_VB_WS, 39, 39, N, DH),
    R(SND_VB_WL, 43, N, 12, DK), R(SND_VB_WS, N, 36, N, DH), R(SND_VB_WL, N, N, 24, DS), R(SND_VB_WS, 46, 34, N, DH),
    R(SND_VB_WL, 46, N, 15, DK), R(SND_VB_WS, N, 39, N, DH), R(SND_VB_WL, N, N, 27, DS), R(SND_VB_WS, 46, 43, N, DH),
    R(SND_VB_WL, 43, N, 15, DK), R(SND_VB_WS, N, 39, N, DH), R(SND_VB_WL, N, N, 27, DS), R(SND_VB_WS, 39, 36, N, DH),
    R(SND_VB_WL, 41, N, 17, DK), R(SND_VB_WS, N, 41, N, DH), R(SND_VB_WL, N, N, 29, DS), R(SND_VB_WS, 39, 44, N, DH),
    R(SND_VB_WL, 36, N, 17, DK), R(SND_VB_WS, N, 41, N, DH), R(SND_VB_WL, N, N, 29, DS), R(SND_VB_WS, 34, 39, N, DH),
    R(SND_VB_WL, 43, N, 19, DK), R(SND_VB_WS, N, 43, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, 46, 46, N, DH),
    R(SND_VB_WL, 46, N, 19, DK), R(SND_VB_WS, 43, 43, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, 46, 46, N, DH),

    /* Bar 29 */
    R(SND_VB_WL, 36, N, 24, DK), R(SND_VB_WS, 39, 36, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, 36, 39, N, DH),
    R(SND_VB_WL, 39, N, 34, DK), R(SND_VB_WS, 41, 36, N, DH), R(SND_VB_WL, 43, N, 31, DS), R(SND_VB_WS, 41, 34, N, DH),

    /* Bar 30 */
    R(SND_VB_WL, 43, N, 24, DK), R(SND_VB_WS, 46, 36, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, 43, 39, N, DH),
    R(SND_VB_WL, 39, N, 34, DK), R(SND_VB_WS, 41, 36, N, DH), R(SND_VB_WL, 36, N, 31, DS), R(SND_VB_WS, N, 34, N, DH),

    /* Bar 31 */
    R(SND_VB_WL, 34, N, 24, DK), R(SND_VB_WS, 36, 36, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, 39, 39, N, DH),
    R(SND_VB_WL, 43, N, 34, DK), R(SND_VB_WS, 41, 36, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, 36, 34, N, DH),

    /* Bar 32 */
    R(SND_VB_WL, 43, N, 24, DK), R(SND_VB_WS, 46, 36, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, 43, 39, N, DH),
    R(SND_VB_WL, 46, N, 34, DK), R(SND_VB_WS, 43, 36, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, 39, 34, N, DH),

    /* Bar 33 */
    R(SND_VB_WL, 41, N, 17, DK), R(SND_VB_WS, 43, 36, N, DH), R(SND_VB_WL, N, N, 24, DS), R(SND_VB_WS, 41, 39, N, DH),
    R(SND_VB_WL, 46, N, 27, DK), R(SND_VB_WS, 43, 36, N, DH), R(SND_VB_WL, 46, N, 24, DS), R(SND_VB_WS, N, 34, N, DH),

    /* Bar 34 */
    R(SND_VB_WL, 46, N, 17, DK), R(SND_VB_WS, 43, 36, N, DH), R(SND_VB_WL, N, N, 24, DS), R(SND_VB_WS, 46, 39, N, DH),
    R(SND_VB_WL, 43, N, 27, DK), R(SND_VB_WS, 41, 36, N, DH), R(SND_VB_WL, 41, N, 24, DS), R(SND_VB_WS, N, 34, N, DH),

    /* Bar 35 */
    R(SND_VB_WL, 39, N, 24, DK), R(SND_VB_WS, 41, 36, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, 43, 39, N, DH),
    R(SND_VB_WL, 43, N, 34, DK), R(SND_VB_WS, 46, 36, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, N, 34, N, DH),

    /* Bar 36 */
    R(SND_VB_WL, 46, N, 24, DK), R(SND_VB_WS, N, 36, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, N, 39, N, DH),
    R(SND_VB_WL, 36, N, 34, DK), R(SND_VB_WS, 39, 36, N, DH), R(SND_VB_WL, N, N, 31, DS), R(SND_VB_WS, N, 34, N, DH),

    /* Bar 37 */
    R(SND_VB_WL, 46, N, 24, DK), R(SND_VB_WS, 43, 36, N, DH), R(SND_VB_WL, 41, N, 31, DS), R(SND_VB_WS, 39, 39, N, DH),
    R(SND_VB_WL, 36, N, 34, DK), R(SND_VB_WS, 34, 36, N, DH), R(SND_VB_WL, 31, N, 31, DS), R(SND_VB_WS, 34, 34, N, DH),

    /* Bar 38 */
    R(SND_VB_WL, 36, N, 24, DK), R(SND_VB_WS, N, 36, N, DH), R(SND_VB_WL, 39, N, 31, DS), R(SND_VB_WS, N, 39, N, DH),
    R(SND_VB_WL, 41, N, 34, DK), R(SND_VB_WS, N, 36, N, DH), R(SND_VB_WL, 46, N, 31, DS), R(SND_VB_WS, N, 34, N, DH),

    /* Bar 39 */
    R(SND_VB_WL, 46, N, 17, DK), R(SND_VB_WS, 43, 36, N, DH), R(SND_VB_WL, 41, N, 24, DS), R(SND_VB_WS, 39, 39, N, DH),
    R(SND_VB_WL, 36, N, 27, DK), R(SND_VB_WS, 34, 36, N, DH), R(SND_VB_WL, 31, N, 24, DS), R(SND_VB_WS, 34, 34, N, DH),

    /* Bar 40 */
    R(SND_VB_WL, 41, N, 19, DK), R(SND_VB_WS, N, 43, N, DH), R(SND_VB_WL, 43, N, 31, DS), R(SND_VB_WS, N, 46, N, DH),
    R(SND_VB_WS, 46, N, 43, DS), R(SND_VB_WS, N, N, N, DS), R(SND_VB_WS, 46, N, N, DS), R(SND_VB_WL, 46, N, 31, DS),
    R(SND_VB_WS, N, N, N, DK),

    SND_VB_LOOP
};

__prg_rom u8 SND_VB_GAMEOVER_BGM[] = {
    /* Bar 1: chromatic stumble downward */
    R(SND_VB_WL, 48, N, 36, DK), R(SND_VB_WS, N, 48, N, DH),
    R(SND_VB_WL, 47, N, 35, DS), R(SND_VB_WS, N, 47, N, DH),
    R(SND_VB_WL, 46, N, 34, DK), R(SND_VB_WS, N, 46, N, DH),
    R(SND_VB_WL, 45, N, 33, DS), R(SND_VB_WS, N, 45, N, DH),

    /* Bar 2: keeps falling */
    R(SND_VB_WL, 44, N, 32, DK), R(SND_VB_WS, N, 44, N, DH),
    R(SND_VB_WL, 43, N, 31, DS), R(SND_VB_WS, N, 43, N, DH),
    R(SND_VB_WL, 42, N, 30, DK), R(SND_VB_WS, N, 42, N, DH),
    R(SND_VB_WL, 41, N, 29, DS), R(SND_VB_WS, N, 41, N, DH),

    /* Bar 3: heavy fall and silent pause */
    R(33, 39, N, 27, DK),
    R(33, 34, N, 22, DS),
    R(66, N, N, N, N),

    /* Bar 4: shamisen-style punchline */
    R(11, 24, N, 12, DK),
    R(11, 31, N, N, N),
    R(110, 36, 36, N, DK),

    SND_VB_END
};

/* Copyright (c) 2026 DAISUKE OBA. SPDX-License-Identifier: MIT.
   Banked standard-driver sound effects. Each pair encodes a note and its parameter; a zero
   note terminates the effect. */

/* Standard-driver SFX are (note, envelope) pairs, terminated by a zero note.
   Audio_Update consumes them; these are not five-byte VBlank music records. */
__prg_rom u8 SND_SFX_CRASH[] = {
    36, 0xF1,
    31, 0xD1,
    26, 0xB1,
    21, 0x91,
    16, 0x71,
    0
};

__prg_rom u8 SND_SFX_MINE_EXPLODE[] = {
    24, 0xF2,
    19, 0xD2,
    16, 0xB2,
    12, 0x92,
     8, 0x62,
    0
};

__prg_rom u8 SND_SFX_EAT[] = {
    40, 0x81,
    48, 0xA1,
    55, 0xC1,
    64, 0xE1,
    0
};

/* Copyright (c) 2026 DAISUKE OBA. SPDX-License-Identifier: MIT.
   Select fixed bank 0 for resident system/audio work and define the short interrupt callback
   plus its byte-sized shared flags. */

#pragma bank 0
#include "shirohebi_input_shared.h"

/* Keep VBlank-facing system helpers resident. */
__hram u8 Audio_NoiseEffectActive;
__wram u8 SND_DemoVBlankCounter;
__wram u8 SND_SystemButtonsSample;
__wram u8 SND_TitleShortcutEnabled;
__wram u8 SND_TitleShortcutPending;
__wram u8 SND_PauseSelectEnabled;
__wram u8 SND_PauseSelectPending;

/* Resident, bounded-time ISR callback. No drawing or banked calls here.
   The raw button row is A=1, B=2, SELECT=4, START=8. Keep the settling
   reads and restore P1 selection: main can be interrupted during __readpadex. */
/* In fixed ROM, sample and restore the P1 button-row selector, latch permitted shortcuts and
   increment the demo clock; never draw or call banked code here. */
void SND_DemoVBlankHook()
{
    __asm {
        LDH_A_MEM 0x00
        AND_IMM 0x30
        PUSH_AF
        LD_A_IMM 0x10
        LDH_MEM_A 0x00
        LDH_A_MEM 0x00
        LDH_A_MEM 0x00
        LDH_A_MEM 0x00
        LDH_A_MEM 0x00
        LDH_A_MEM 0x00
        LDH_A_MEM 0x00
        CPL
        AND_IMM 0x0F
        LD_MEM_A SND_SystemButtonsSample
        POP_AF
        LDH_MEM_A 0x00
    }

    /* Latch only the title shortcut while the title explicitly allows it.
       A momentary SELECT+START survives a blocking title/VRAM update. */
    if (SND_TitleShortcutEnabled != 0) {
        if ((SND_SystemButtonsSample & 0x0C) == 0x0C) {
            SND_TitleShortcutPending = 1;
        }
    }

    /* A single SELECT sample is sufficient after the pause-entry release
       gate arms this shortcut. Retain a short tap across blocked main/VRAM
       work. START takes precedence in SND_TickPause; no hold timer remains. */
    if (SND_PauseSelectEnabled != 0 &&
        (SND_SystemButtonsSample & 0x0C) == 0x04) {
        SND_PauseSelectPending = 1;
    }
    SND_DemoVBlankCounter = (u8)(SND_DemoVBlankCounter + 1);
}

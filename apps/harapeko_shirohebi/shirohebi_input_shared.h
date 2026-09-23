/* Copyright (c) 2026 DAISUKE OBA. SPDX-License-Identifier: MIT.
   Declarations shared by the fixed-bank VBlank callback and banked main-loop input code.
   Enabled flags control which pending events the ISR may latch. */

#ifndef SHIROHEBI_INPUT_SHARED_H
#define SHIROHEBI_INPUT_SHARED_H

/* Single-press event flags shared with the fixed-bank VBlank sampler.
   Pause SELECT has no duration threshold. */

extern __wram u8 SND_DemoVBlankCounter;
extern __wram u8 SND_SystemButtonsSample;
extern __wram u8 SND_TitleShortcutEnabled;
extern __wram u8 SND_TitleShortcutPending;
extern __wram u8 SND_PauseSelectEnabled;
extern __wram u8 SND_PauseSelectPending;
void SND_DemoVBlankHook();

#endif

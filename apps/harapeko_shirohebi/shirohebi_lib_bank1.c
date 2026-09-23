/* Copyright (c) 2026 DAISUKE OBA. SPDX-License-Identifier: MIT.
   Select switchable bank 1 for following support-library compilation units. File order in
   build.ps1 is part of the ROM layout. */

#pragma bank 1

/* Keep library sources after audio.c out of the fixed bank. */

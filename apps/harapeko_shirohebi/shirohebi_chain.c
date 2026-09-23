/* Copyright (c) 2026 DAISUKE OBA. SPDX-License-Identifier: MIT.
   Compile the library follower and all of its internal helpers together in switchable bank 2,
   then restore bank 1 for subsequent library inputs. */

/* Keep the complete follower in one bank: per-joint helper calls must not
   switch ROM banks. The implementation comes directly from the library. */
#pragma bank 2
#include "chain.c"
#pragma bank 1

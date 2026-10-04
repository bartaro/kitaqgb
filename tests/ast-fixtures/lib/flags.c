#include "rpg.h"

// RAM-backed game state: 2048 flag bits and 64 quest-state bytes. Persistence
// requires an explicit game save path; these arrays do not write cartridge RAM.
static u8 flags_bits[256];
static u8 quest_states[64];

// Read a bit from the flag array. Callers must keep id below 2048.
u8 flag_get(u16 id)
{
    return __bit_test(flags_bits, id);
}

// Set a flag bit without disturbing adjacent flags; id must be below 2048.
void flag_set(u16 id)
{
    __bit_set(flags_bits, id);
}

// Clear a flag bit; id must be below 2048.
void flag_clear(u16 id)
{
    __bit_clear(flags_bits, id);
}

// Read a quest-state byte; quest_id must be below 64.
u8 quest_state(u8 quest_id)
{
    return quest_states[quest_id];
}

// Replace a quest-state byte without validating its game-specific meaning.
// The caller must keep quest_id below 64.
void quest_set_state(u8 quest_id, u8 state)
{
    quest_states[quest_id] = state;
}

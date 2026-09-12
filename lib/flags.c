#include "rpg.h"

static u8 flags_bits[256];
static u8 quest_states[64];

u8 flag_get(u16 id)
{
    return __bit_test(flags_bits, id);
}

void flag_set(u16 id)
{
    __bit_set(flags_bits, id);
}

void flag_clear(u16 id)
{
    __bit_clear(flags_bits, id);
}

u8 quest_state(u8 quest_id)
{
    return quest_states[quest_id];
}

void quest_set_state(u8 quest_id, u8 state)
{
    quest_states[quest_id] = state;
}

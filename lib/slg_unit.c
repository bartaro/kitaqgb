#include "rpg.h"

// A unit can act only while HP is nonzero and its acted flag is clear.
// The unit pointer must refer to a valid record.
u8 unit_can_act(const unit_t* u)
{
    if (u->hp == 0) return 0;
    if (u->acted != 0) return 0;
    return 1;
}

// Build movement costs, then convert reachable cells to one and unreachable
// 0xFF cells to zero. The caller provides one output byte per current-map cell
// and a valid unit origin; path-search workspace limits also apply.
u8 unit_move_range(unit_t* u, u8* out_mask)
{
    u8 w = map_current_width();
    u8 h = map_current_height();
    u16 total;
    u16 i;

    if (w == 0 || h == 0) return 0;

    // A valid origin is required: the fill leaves storage unchanged on invalid origins, but this loop still converts it.
    range_fill_move(u->x, u->y, u->move, out_mask);
    total = (u16)((u16)w * (u16)h);
    i = 0;
    while (i < total) {
        out_mask[i] = (u8)((out_mask[i] == 0xFF) ? 0 : 1);
        i++;
    }
    return 1;
}

// Mark cells whose Manhattan distance falls within the inclusive attack range.
// This geometric mask does not test obstacles, line of sight or other units.
// Keep all evaluated Manhattan distances within 0..255 because the intrinsic result is a byte.
u8 unit_attack_range(unit_t* u, u8* out_mask)
{
    u8 w = map_current_width();
    u8 h = map_current_height();
    u8 y;

    if (w == 0 || h == 0) return 0;

    __memset(out_mask, 0, (u16)((u16)w * (u16)h));
    y = 0;
    while (y < h) {
        u8 x = 0;
        while (x < w) {
            u8 dist = __manhattan(u->x, u->y, x, y);
            if (dist >= u->atk_min && dist <= u->atk_max) {
                out_mask[__map_index(x, y, w)] = 1;
            }
            x++;
        }
        y++;
    }
    return 1;
}

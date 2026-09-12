#include "rpg.h"

u8 unit_can_act(const unit_t* u)
{
    if (u->hp == 0) return 0;
    if (u->acted != 0) return 0;
    return 1;
}

u8 unit_move_range(unit_t* u, u8* out_mask)
{
    u8 w = map_current_width();
    u8 h = map_current_height();
    u16 total;
    u16 i;

    if (w == 0 || h == 0) return 0;

    range_fill_move(u->x, u->y, u->move, out_mask);
    total = (u16)((u16)w * (u16)h);
    i = 0;
    while (i < total) {
        out_mask[i] = (u8)((out_mask[i] == 0xFF) ? 0 : 1);
        i++;
    }
    return 1;
}

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

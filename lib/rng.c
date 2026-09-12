#include "rpg.h"

u8 rng8()
{
    return __rng8();
}

void rng_seed(u16 seed)
{
    __rng_seed(seed);
}

u8 rng_next8()
{
    return rng8();
}

u16 rng16()
{
    u16 hi = (u16)rng8();
    u16 lo = (u16)rng8();
    return (u16)((hi << 8) | lo);
}

u16 rng_next16()
{
    return rng16();
}

u8 rand_range(u8 max)
{
    if (max == 0) return 0;
    return (u8)(rng8() % max);
}

u8 rng_range(u8 max)
{
    return rand_range(max);
}

u8 rng_chance(u8 percent)
{
    if (percent == 0) return 0;
    if (percent >= 100) return 1;
    return (u8)(rng_range(100) < percent);
}

u8 weighted_choice(const u8* weights, u8 count)
{
    u8 i;
    u16 total;
    u16 pick;
    u16 acc;

    total = 0;
    i = 0;
    while (i < count) {
        total = (u16)(total + (u16)weights[i]);
        i++;
    }

    if (total == 0) return 0;

    pick = (u16)(rng16() % total);
    acc = 0;
    i = 0;
    while (i < count) {
        acc = (u16)(acc + (u16)weights[i]);
        if (pick < acc) return i;
        i++;
    }

    return (u8)(count - 1);
}

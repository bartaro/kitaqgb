#include "rpg.h"

// Advance the compiler-provided pseudorandom generator and return one byte.
u8 rng8()
{
    return __rng8();
}

// Replace the generator seed so subsequent calls can reproduce a sequence.
void rng_seed(u16 seed)
{
    __rng_seed(seed);
}

// Alias the byte generator; this consumes one generator step.
u8 rng_next8()
{
    return rng8();
}

// Consume two generator bytes in high-byte/low-byte order to form a 16-bit value.
u16 rng16()
{
    u16 hi = (u16)rng8();
    u16 lo = (u16)rng8();
    return (u16)((hi << 8) | lo);
}

// Alias the two-step 16-bit generator.
u16 rng_next16()
{
    return rng16();
}

// Reduce a random byte modulo max; zero returns zero without consuming randomness.
// Modulo reduction is biased unless max divides the generator range.
u8 rand_range(u8 max)
{
    if (max == 0) return 0;
    return (u8)(rng8() % max);
}

// Alias rand_range and return a value in [0,max), or zero when max is zero.
u8 rng_range(u8 max)
{
    return rand_range(max);
}

// Return false for zero and true for values at least 100. Intermediate values
// use the modulo-based range generator, so percentages are approximate.
u8 rng_chance(u8 percent)
{
    if (percent == 0) return 0;
    if (percent >= 100) return 1;
    return (u8)(rng_range(100) < percent);
}

// Select an index from cumulative byte weights using a 16-bit random draw.
// A zero total, including an empty list, returns zero; callers must distinguish
// that case from a valid index. Modulo reduction can introduce a small bias.
u8 weighted_choice(const u8* weights, u8 count)
{
    u8 i;
    u16 total;
    u16 pick;
    u16 acc;

    // At most 255 byte weights sum to 65025, so this accumulator fits u16.
    // A nonempty list requires count readable weight bytes.
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

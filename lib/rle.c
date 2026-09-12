#include "rpg.h"

u16 rle_decode(void* dst, const void* src)
{
    const u8* in = (const u8*)src;
    u8* out = (u8*)dst;
    u16 total = 0;

    while (1) {
        u8 count = *in++;
        u8 value;
        if (count == 0) break;
        value = *in++;
        while (count != 0) {
            *out++ = value;
            total++;
            count--;
        }
    }

    return total;
}

u16 rle_decode_far(void* dst, u8 bank, const void* src)
{
    const u8* p = (const u8*)src;
    u8* out = (u8*)dst;
    u16 total = 0;

    while (1) {
        u8 count = __farpeek8(bank, p);
        u8 value;
        p++;
        if (count == 0) break;
        value = __farpeek8(bank, p);
        p++;
        while (count != 0) {
            *out++ = value;
            total++;
            count--;
        }
    }

    return total;
}

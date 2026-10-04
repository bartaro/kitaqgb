#include "rpg.h"

// Expand count/value byte pairs until a zero count terminates the stream.
// The caller must provide a valid terminator and sufficient destination storage;
// there are no source or destination bounds parameters. Return the 16-bit byte count.
// Use writable RAM for the output, keep input/output from overlapping, and keep the
// expanded size within u16. This decoder does not synchronize writes to VRAM.
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

// Decode the same count/value format using bank-qualified reads for every source
// byte. Keep the encoded stream within the supplied bank/address mapping and
// provide enough destination space; a zero count ends the stream.
// The bank argument stays fixed while the source address advances; no bank-boundary rollover occurs.
// Output must be writable RAM with a representable expanded size, as in rle_decode.
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

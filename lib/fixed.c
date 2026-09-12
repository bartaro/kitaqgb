#include "fixed.h"

fix8 fix_from_int(s16 n)
{
    return (fix8)(n << 8);
}

s16 fix_to_int(fix8 x)
{
    return (s16)(x >> 8);
}

fix8 fix_mul(fix8 a, fix8 b)
{
    return (fix8)((a >> 4) * (b >> 4));
}

fix8 fix_div(fix8 a, fix8 b)
{
    if (b == 0) return 0;
    return (fix8)(((a >> 4) * 256) / (b >> 4));
}

fix8 fix_lerp(fix8 a, fix8 b, u8 t_q8)
{
    fix8 d = (fix8)(b - a);
    return (fix8)(a + (fix8)((d >> 4) * (s16)(t_q8 >> 4)));
}

s16 kq_abs_s16(s16 v)
{
    if (v < 0) return (s16)(0 - v);
    return v;
}

s16 kq_min_s16(s16 a, s16 b)
{
    if (a < b) return a;
    return b;
}

s16 kq_max_s16(s16 a, s16 b)
{
    if (a > b) return a;
    return b;
}

s16 kq_clamp_s16(s16 v, s16 lo, s16 hi)
{
    if (v < lo) return lo;
    if (v > hi) return hi;
    return v;
}

u8 kq_rect_intersect(KQRect a, KQRect b)
{
    if ((s16)(a.x + a.w) <= b.x) return 0;
    if ((s16)(b.x + b.w) <= a.x) return 0;
    if ((s16)(a.y + a.h) <= b.y) return 0;
    if ((s16)(b.y + b.h) <= a.y) return 0;
    return 1;
}

u8 kq_point_in_rect(s16 x, s16 y, KQRect r)
{
    if (x < r.x) return 0;
    if (y < r.y) return 0;
    if (x >= (s16)(r.x + r.w)) return 0;
    if (y >= (s16)(r.y + r.h)) return 0;
    return 1;
}


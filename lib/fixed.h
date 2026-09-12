#pragma once

typedef s16 fix8;

#define FIX_ONE ((fix8)256)
#define FIX_HALF ((fix8)128)

typedef __packed struct {
    fix8 x;
    fix8 y;
} Vec2;

typedef __packed struct {
    s16 x;
    s16 y;
    s16 w;
    s16 h;
} KQRect;

fix8 fix_from_int(s16 n);
s16 fix_to_int(fix8 x);
fix8 fix_mul(fix8 a, fix8 b);
fix8 fix_div(fix8 a, fix8 b);
fix8 fix_lerp(fix8 a, fix8 b, u8 t_q8);
s16 kq_abs_s16(s16 v);
s16 kq_min_s16(s16 a, s16 b);
s16 kq_max_s16(s16 a, s16 b);
s16 kq_clamp_s16(s16 v, s16 lo, s16 hi);
u8 kq_rect_intersect(KQRect a, KQRect b);
u8 kq_point_in_rect(s16 x, s16 y, KQRect r);


#pragma once

// Signed Q8.8 uses 256 units per pixel/value: the range is -128 to 127 + 255/256.
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

// Convert an integer to signed 8.8 fixed point by moving it into the high byte.
// The caller must keep the result within the representable 16-bit range.
fix8 fix_from_int(s16 n);
// Discard the eight fractional bits using a signed right shift.
s16 fix_to_int(fix8 x);
// Approximate 8.8 multiplication by discarding four low bits from each operand
// before multiplying. This trades precision for a smaller intermediate product.
fix8 fix_mul(fix8 a, fix8 b);
// Scale the reduced-precision quotient back to 8.8 and return zero for b == 0.
// Callers must also ensure (b >> 4) is nonzero; the explicit check only tests b.
// The intermediate product must fit the compiler's integer arithmetic.
fix8 fix_div(fix8 a, fix8 b);
// Interpolate toward b using a quantized Q0.8 weight. Both shifts discard low
// bits, so small changes may vanish and a weight of 255 need not reach b.
fix8 fix_lerp(fix8 a, fix8 b, u8 t_q8);
// Return the magnitude in s16. The most-negative s16 has no positive s16
// representation and must be excluded by callers needing a nonnegative result.
s16 kq_abs_s16(s16 v);
// Select the smaller signed value.
s16 kq_min_s16(s16 a, s16 b);
// Select the larger signed value.
s16 kq_max_s16(s16 a, s16 b);
// Clamp to an inclusive interval; callers must provide lo <= hi.
s16 kq_clamp_s16(s16 v, s16 lo, s16 hi);
// Test overlap using exclusive right/bottom edges; touching edges do not collide.
// Require positive dimensions and edge sums that fit s16; zero-sized rectangles
// are not explicitly rejected and may be reported as overlapping.
u8 kq_rect_intersect(KQRect a, KQRect b);
// Test a half-open rectangle: left/top edges are included, right/bottom excluded.
// The coordinate-plus-size sums must remain representable as s16.
u8 kq_point_in_rect(s16 x, s16 y, KQRect r);


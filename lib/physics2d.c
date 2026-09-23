#include "physics2d.h"

u16 __mul16x8(u16 a, u8 b);

// Scale by a signed Q8 coefficient in -256..256, truncating the magnitude
// before restoring the sign. The assembly handles zero and +/-256 separately;
// coefficients outside this interval are not supported.
s16 kq2d_scale_q8(s16 value, s16 coefficient)
{
    // Eight-bit shift/add product in HL:C, with sign applied after truncation.
    __asm {
        LD_HL_IMM value
        LD_A_HL
        LD_E_A
        INC_HL
        LD_A_HL
        LD_D_A
        OR_E
        JP_Z kqscale_zero
        LD_HL_IMM coefficient
        LD_A_HL
        LD_C_A
        INC_HL
        LD_A_HL
        LD_B_A
        LD_A_C
        OR_A
        JR_NZ kqscale_fraction
        LD_A_B
        OR_A
        JP_Z kqscale_zero
        LD_H_D
        LD_L_E
        AND_IMM 128
        RET_Z
        JP kqscale_negate
kqscale_fraction:
        LD_A_D
        XOR_B
        AND_IMM 128
        PUSH_AF
        LD_A_D
        AND_IMM 128
        JR_Z kqscale_value_positive
        XOR_A
        SUB_E
        LD_E_A
        LD_A_IMM 0
        SBC_D
        LD_D_A
kqscale_value_positive:
        LD_A_B
        AND_IMM 128
        JR_Z kqscale_coefficient_positive
        XOR_A
        SUB_C
        LD_C_A
kqscale_coefficient_positive:
        LD_HL_IMM 0
        // Unroll eight bits: avoid a loop branch on every fixed-point product.
        LD_A_C
        AND_IMM 1
        JR_Z kqscale_shift_0
        ADD_HL_DE
kqscale_shift_0:
        LD_A_H
        RRA
        LD_H_A
        LD_A_L
        RRA
        LD_L_A
        LD_A_C
        RRA
        LD_C_A
        LD_A_C
        AND_IMM 1
        JR_Z kqscale_shift_1
        ADD_HL_DE
kqscale_shift_1:
        LD_A_H
        RRA
        LD_H_A
        LD_A_L
        RRA
        LD_L_A
        LD_A_C
        RRA
        LD_C_A
        LD_A_C
        AND_IMM 1
        JR_Z kqscale_shift_2
        ADD_HL_DE
kqscale_shift_2:
        LD_A_H
        RRA
        LD_H_A
        LD_A_L
        RRA
        LD_L_A
        LD_A_C
        RRA
        LD_C_A
        LD_A_C
        AND_IMM 1
        JR_Z kqscale_shift_3
        ADD_HL_DE
kqscale_shift_3:
        LD_A_H
        RRA
        LD_H_A
        LD_A_L
        RRA
        LD_L_A
        LD_A_C
        RRA
        LD_C_A
        LD_A_C
        AND_IMM 1
        JR_Z kqscale_shift_4
        ADD_HL_DE
kqscale_shift_4:
        LD_A_H
        RRA
        LD_H_A
        LD_A_L
        RRA
        LD_L_A
        LD_A_C
        RRA
        LD_C_A
        LD_A_C
        AND_IMM 1
        JR_Z kqscale_shift_5
        ADD_HL_DE
kqscale_shift_5:
        LD_A_H
        RRA
        LD_H_A
        LD_A_L
        RRA
        LD_L_A
        LD_A_C
        RRA
        LD_C_A
        LD_A_C
        AND_IMM 1
        JR_Z kqscale_shift_6
        ADD_HL_DE
kqscale_shift_6:
        LD_A_H
        RRA
        LD_H_A
        LD_A_L
        RRA
        LD_L_A
        LD_A_C
        RRA
        LD_C_A
        LD_A_C
        AND_IMM 1
        JR_Z kqscale_shift_7
        ADD_HL_DE
kqscale_shift_7:
        LD_A_H
        RRA
        LD_H_A
        LD_A_L
        RRA
        LD_L_A
        LD_A_C
        RRA
        LD_C_A
        POP_AF
        OR_A
        RET_Z
kqscale_negate:
        XOR_A
        SUB_L
        LD_L_A
        LD_A_IMM 0
        SBC_H
        LD_H_A
        RET
kqscale_zero:
        LD_HL_IMM 0
        RET
    }
}

// Fractional long division avoids overflowing numerator * 256 on the LR35902.
// Return floor(256*numerator/denominator), capped at 256; zero denominator
// returns zero. Keep denominator <= 32767 so doubling the remainder fits 16 bits.
u16 kq2d__fraction_q8(u16 numerator, u16 denominator)
{
    __asm {
        LD_HL_IMM denominator
        LD_A_HL
        LD_E_A
        INC_HL
        LD_A_HL
        LD_D_A
        OR_E
        LD_HL_IMM 0
        RET_Z
        LD_HL_IMM numerator
        LD_A_HL
        INC_HL
        LD_H_HL
        LD_L_A
        LD_A_H
        CP_D
        JR_C kqfrac_begin
        JR_NZ kqfrac_one
        LD_A_L
        CP_E
        JR_C kqfrac_begin
kqfrac_one:
        LD_HL_IMM 256
        RET
kqfrac_begin:
        LD_C_IMM 0
        LD_B_IMM 8
kqfrac_loop:
        ADD_HL_HL
        LD_A_C
        ADD_A
        LD_C_A
        LD_A_H
        CP_D
        JR_C kqfrac_next
        JR_NZ kqfrac_subtract
        LD_A_L
        CP_E
        JR_C kqfrac_next
kqfrac_subtract:
        LD_A_L
        SUB_E
        LD_L_A
        LD_A_H
        SBC_D
        LD_H_A
        INC_C
kqfrac_next:
        DEC_B
        JR_NZ kqfrac_loop
        LD_L_C
        LD_H_IMM 0
        RET
    }
}

// ceil(256 * sqrt(1 + (i/64)^2)) - 256. Round the ratio up as well.
__prg_rom u8 kq2d_hypot_extra[] = {
    0,1,1,1,1,1,2,2,2,3,4,4,5,6,7,7,8,9,10,12,13,14,15,17,18,19,21,22,24,26,27,29,31,33,34,36,38,40,42,44,46,49,51,53,55,57,60,62,64,67,69,72,74,77,79,82,85,87,90,93,95,98,101,104,107
};

// Reduce both velocity components with one conservative length estimate.
// A null body is ignored; a nonpositive limit zeros velocity. This helper ignores
// active/mass flags. Components and positive limits must have magnitude <= 16383.
void kq2d_body_limit_speed(KQBody2D* body, s16 max_speed)
{
    u16 x;
    u16 y;
    u16 swap;
    u16 ratio;
    u16 length;
    if (body == 0) return;
    if (max_speed <= 0) { body->vx = 0; body->vy = 0; return; }
    x = (u16)body->vx;
    y = (u16)body->vy;
    if (body->vx < 0) x = (u16)(0 - body->vx);
    if (body->vy < 0) y = (u16)(0 - body->vy);
    if (x + y <= (u16)max_speed) return;
    if (x < y) { swap = x; x = y; y = swap; }
    // major + minor/2 bounds hypot from above for 0 <= minor <= major.
    if (x + (y >> 1) + 1 <= (u16)max_speed) return;
    // Round the minor/major ratio upward for the 65-entry upper-bound table.
    // The extra unit covers truncation when converting the estimate back to speed.
    ratio = (kq2d__fraction_q8(y, x) + 4) >> 2;
    if (ratio > 64) ratio = 64;
    length = x + (u16)kq2d_scale_q8((s16)x, (s16)kq2d_hypot_extra[(__safe_index u8)ratio]) + 1;
    if (length <= (u16)max_speed) return;
    ratio = kq2d__fraction_q8((u16)max_speed, length);
    body->vx = kq2d_scale_q8(body->vx, (s16)ratio);
    body->vy = kq2d_scale_q8(body->vy, (s16)ratio);
}

// Estimate a linear gap crossing: initial contact returns 0, and an endpoint
// still outside or exactly on the surface returns 256. Otherwise interpolate in
// Q8. Keep both signed gaps within +/-8191; no body position is changed.
u16 kq2d_surface_toi_q8(s16 start_gap, s16 end_gap)
{
    if (start_gap <= 0) return 0;
    if (end_gap >= 0) return 256;
    return kq2d__fraction_q8((u16)start_gap, (u16)(start_gap - end_gap));
}

// Retain sub-unit gravity along a shallow support without rounding every fast hit.
// For small resting velocities, scale at extra precision and round halves away
// from zero. Values outside -63..63 use the ordinary truncating Q8 helper.
s16 kq2d__rest_component(s16 value, s16 normal)
{
    s16 product;
    if(value <= -64 || value >= 64) return kq2d_scale_q8(value,normal);
    product = kq2d_scale_q8((s16)(value*128),normal);
    if(product < 0) return (s16)(0 - ((64-product) >> 7));
    return (s16)((product+64) >> 7);
}

// Resolve incoming velocity relative to a moving surface and return the
// positive incoming normal speed; null inputs or separating motion return zero.
// Use a Q8 unit normal, nonnegative threshold/kick, and relative components
// within +/-8191. This does not detect contact, move positions, or check body mass.
s16 kq2d_body_resolve_surface(KQBody2D* body, const KQSurface2D* surface)
{
    s16 vx;
    s16 vy;
    s16 speed;
    s16 tangent;
    s16 bounce = 0;
    s16 impulse;
    s16 friction;
    if (body == 0 || surface == 0) return 0;
    vx = (s16)(body->vx - surface->vx);
    vy = (s16)(body->vy - surface->vy);
    speed = (s16)(kq2d_scale_q8(vx,surface->nx_q8) + kq2d_scale_q8(vy,surface->ny_q8));
    if (speed >= 0) return 0;
    speed = (s16)(0 - speed);
    if (speed > surface->bounce_threshold) bounce = kq2d_scale_q8(speed,(s16)surface->restitution_q8);
    if (bounce < surface->kick) bounce = surface->kick;
    impulse = (s16)(speed + bounce);
    // A resting contact is a tangent constraint, not a tiny rounded bounce.
    if (bounce == 0) {
        tangent = (s16)(kq2d__rest_component(vy,surface->nx_q8) - kq2d__rest_component(vx,surface->ny_q8));
        friction = kq2d_scale_q8(impulse,(s16)surface->friction_q8);
        if (tangent > friction) tangent = (s16)(tangent - friction);
        else if (tangent < (s16)(0 - friction)) tangent = (s16)(tangent + friction);
        else tangent = 0;
        body->vx = (s16)(surface->vx - kq2d__rest_component(tangent,surface->ny_q8));
        body->vy = (s16)(surface->vy + kq2d__rest_component(tangent,surface->nx_q8));
        return speed;
    }
    body->vx = (s16)(body->vx + kq2d_scale_q8(impulse,surface->nx_q8));
    body->vy = (s16)(body->vy + kq2d_scale_q8(impulse,surface->ny_q8));
    if (surface->friction_q8 != 0) {
        tangent = (s16)(kq2d_scale_q8(vy,surface->nx_q8) - kq2d_scale_q8(vx,surface->ny_q8));
        friction = kq2d_scale_q8(impulse,(s16)surface->friction_q8);
        if (tangent > friction) tangent = friction;
        if (tangent < (s16)(0 - friction)) tangent = (s16)(0 - friction);
        body->vx = (s16)(body->vx + kq2d_scale_q8(tangent,surface->ny_q8));
        body->vy = (s16)(body->vy - kq2d_scale_q8(tangent,surface->nx_q8));
    }
    return speed;
}

// Shared operands avoid repeated C arithmetic in the hot integration path.
// Do not call these helpers from an interrupt while another physics call is active.
s16 kq2d_asm_value;
s16 kq2d_asm_operand;
s16 kq2d_asm_limit;
s16 kq2d_asm_tmp;

// Add the shared operand to the shared value with carry between bytes.
// The 16-bit result wraps; this routine does not saturate on signed overflow.
void kq2d_add_s16_asm()
{
    __asm {
        LD_HL_IMM kq2d_asm_value
        LD_A_HL
        LD_B_A
        INC_HL
        LD_A_HL
        LD_C_A

        LD_HL_IMM kq2d_asm_operand
        LD_A_HL
        ADD_B
        LD_B_A
        INC_HL
        LD_A_HL
        ADC_C
        LD_C_A

        LD_HL_IMM kq2d_asm_value
        LD_A_B
        LDI_HL_A
        LD_A_C
        LD_HL_A
        RET
    }
}

// Negate a negative shared value in place using two's complement.
// The minimum signed value stays 0x8000, which is not a positive s16 magnitude.
void kq2d_abs_s16_asm()
{
    __asm {
        LD_HL_IMM kq2d_asm_value
        INC_HL
        LD_A_HL
        AND_IMM 0x80
        JP_Z kq2dabs_ret

        LD_HL_IMM kq2d_asm_value
        LD_A_HL
        CPL
        ADD_A_IMM 1
        LD_HL_A
        INC_HL
        LD_A_HL
        CPL
        ADC_IMM 0
        LD_HL_A

kq2dabs_ret:
        RET
    }
}

// Clamp the shared signed value to +/- the shared nonnegative limit. Compare
// magnitude high bytes first, then low bytes; negative inputs use shared scratch.
void kq2d_clamp_abs_s16_asm()
{
    __asm {
        LD_HL_IMM kq2d_asm_value
        INC_HL
        LD_A_HL
        AND_IMM 0x80
        JP_NZ kq2dcl_neg

kq2dcl_pos:
        LD_HL_IMM kq2d_asm_value
        INC_HL
        LD_A_HL
        LD_B_A
        LD_HL_IMM kq2d_asm_limit
        INC_HL
        LD_A_HL
        LD_C_A
        LD_A_B
        CP_C
        JP_C kq2dcl_done
        JP_Z kq2dcl_pos_low
        JP kq2dcl_set_pos

kq2dcl_pos_low:
        LD_HL_IMM kq2d_asm_value
        LD_A_HL
        LD_B_A
        LD_HL_IMM kq2d_asm_limit
        LD_A_HL
        LD_C_A
        LD_A_B
        CP_C
        JP_C kq2dcl_done
        JP_Z kq2dcl_done

kq2dcl_set_pos:
        LD_HL_IMM kq2d_asm_limit
        LD_A_HL
        LD_B_A
        INC_HL
        LD_A_HL
        LD_C_A
        LD_HL_IMM kq2d_asm_value
        LD_A_B
        LDI_HL_A
        LD_A_C
        LD_HL_A
        JP kq2dcl_done

kq2dcl_neg:
        LD_HL_IMM kq2d_asm_value
        LD_A_HL
        LD_B_A
        INC_HL
        LD_A_HL
        LD_C_A
        LD_HL_IMM kq2d_asm_tmp
        LD_A_B
        LDI_HL_A
        LD_A_C
        LD_HL_A

        LD_HL_IMM kq2d_asm_tmp
        LD_A_HL
        CPL
        ADD_A_IMM 1
        LD_HL_A
        INC_HL
        LD_A_HL
        CPL
        ADC_IMM 0
        LD_HL_A

        LD_HL_IMM kq2d_asm_tmp
        INC_HL
        LD_A_HL
        LD_B_A
        LD_HL_IMM kq2d_asm_limit
        INC_HL
        LD_A_HL
        LD_C_A
        LD_A_B
        CP_C
        JP_C kq2dcl_done
        JP_Z kq2dcl_neg_low
        JP kq2dcl_set_neg

kq2dcl_neg_low:
        LD_HL_IMM kq2d_asm_tmp
        LD_A_HL
        LD_B_A
        LD_HL_IMM kq2d_asm_limit
        LD_A_HL
        LD_C_A
        LD_A_B
        CP_C
        JP_C kq2dcl_done
        JP_Z kq2dcl_done

kq2dcl_set_neg:
        LD_HL_IMM kq2d_asm_limit
        LD_A_HL
        LD_B_A
        INC_HL
        LD_A_HL
        LD_C_A
        LD_HL_IMM kq2d_asm_value
        LD_A_B
        LDI_HL_A
        LD_A_C
        LD_HL_A

        LD_HL_IMM kq2d_asm_value
        LD_A_HL
        CPL
        ADD_A_IMM 1
        LD_HL_A
        INC_HL
        LD_A_HL
        CPL
        ADC_IMM 0
        LD_HL_A

kq2dcl_done:
        RET
    }
}

// Internal helper: absolute value for signed 16-bit integers.
// Return a magnitude through shared assembly scratch; exclude -32768 when
// a positive s16 result is required. Calls are not reentrant.
s16 kq2d__abs_s16(s16 v)
{
    kq2d_asm_value = v;
    kq2d_abs_s16_asm();
    return kq2d_asm_value;
}

// Retain the caller-owned body array without initializing its elements.
// Set gravity to (0,32), four contact iterations and a per-axis speed limit of 512.
// A null world is ignored; the array must outlive subsequent world operations.
void kq2d_world_init(KQWorld2D* world, KQBody2D* bodies, u8 body_count)
{
    if (world == 0) return;

    world->bodies = bodies;
    world->body_count = body_count;
    world->gravity_x = 0;
    world->gravity_y = 32;
    world->solver_iterations = 4;
    world->max_speed = 512;
}

// Test strict overlap of two centered boxes; touching edges return false.
// Both pointers must be valid. Active/mass flags are ignored, and coordinate
// differences and extent sums must fit the signed 16-bit calculations.
u8 kq2d_overlap_aabb(const KQBody2D* a, const KQBody2D* b)
{
    s16 dx = (s16)(b->x - a->x);
    s16 dy = (s16)(b->y - a->y);

    s16 adx = kq2d__abs_s16(dx);
    s16 ady = kq2d__abs_s16(dy);

    s16 sx = (s16)(a->half_x + b->half_x);
    s16 sy = (s16)(a->half_y + b->half_y);

    if (adx >= sx) return 0;
    if (ady >= sy) return 0;
    return 1;
}

// Initialize an active unit-inverse-mass body at the supplied center and half
// extents, with zero velocity and reserved restitution set to zero. Null is ignored.
void kq2d_body_init(KQBody2D* body, s16 x, s16 y, s16 half_x, s16 half_y)
{
    if (body == 0) return;
    body->x = x;
    body->y = y;
    body->vx = 0;
    body->vy = 0;
    body->half_x = half_x;
    body->half_y = half_y;
    body->inv_mass_q8 = 256;
    body->restitution_q8 = 0;
    body->active = 1;
}

// Replace the center without altering velocity, extents or activity. Null is ignored.
void kq2d_body_set_pos(KQBody2D* body, s16 x, s16 y)
{
    if (body == 0) return;
    body->x = x;
    body->y = y;
}

// Replace velocity without clamping or checking activity/mass. Null is ignored.
void kq2d_body_set_velocity(KQBody2D* body, s16 vx, s16 vy)
{
    if (body == 0) return;
    body->vx = vx;
    body->vy = vy;
}

// Add gravity and clamp each velocity axis separately for an active dynamic
// body. Null, inactive and static bodies are ignored. Supply a nonnegative limit
// and avoid overflow in the additions before clamping; position is unchanged.
void kq2d_body_apply_gravity(KQBody2D* body, s16 gravity_x, s16 gravity_y, s16 max_speed)
{
    if (body == 0) return;
    if (body->active == 0) return;
    if (body->inv_mass_q8 <= 0) return;

    kq2d_asm_value = body->vx;
    kq2d_asm_operand = gravity_x;
    kq2d_add_s16_asm();
    kq2d_asm_limit = max_speed;
    kq2d_clamp_abs_s16_asm();
    body->vx = kq2d_asm_value;

    kq2d_asm_value = body->vy;
    kq2d_asm_operand = gravity_y;
    kq2d_add_s16_asm();
    kq2d_asm_limit = max_speed;
    kq2d_clamp_abs_s16_asm();
    body->vy = kq2d_asm_value;
}

// Scale both velocities through the signed 8-bit coefficient intrinsic.
// Despite the unsigned parameter, values 128..255 become negative coefficients;
// this is not a full-range unsigned Q8 damping helper. Null is ignored.
void kq2d_body_apply_friction(KQBody2D* body, u8 friction_q8)
{
    if (body == 0) return;
    body->vx = (s16)__smul16x8(body->vx, (s8)friction_q8);
    body->vy = (s16)__smul16x8(body->vy, (s8)friction_q8);
}

// For one active dynamic body, add gravity, clamp each velocity axis and then
// add velocity to position. Null inputs are ignored; no contacts are solved.
// Use a nonnegative speed limit and keep additions in signed 16-bit range.
void kq2d_integrate_body(KQWorld2D* world, KQBody2D* body)
{
    if (world == 0) return;
    if (body == 0) return;
    if (body->active == 0) return;
    if (body->inv_mass_q8 <= 0) return;

    kq2d_asm_value = body->vx;
    kq2d_asm_operand = world->gravity_x;
    kq2d_add_s16_asm();
    kq2d_asm_limit = world->max_speed;
    kq2d_clamp_abs_s16_asm();
    body->vx = kq2d_asm_value;

    kq2d_asm_value = body->vy;
    kq2d_asm_operand = world->gravity_y;
    kq2d_add_s16_asm();
    kq2d_asm_limit = world->max_speed;
    kq2d_clamp_abs_s16_asm();
    body->vy = kq2d_asm_value;

    kq2d_asm_value = body->x;
    kq2d_asm_operand = body->vx;
    kq2d_add_s16_asm();
    body->x = kq2d_asm_value;

    kq2d_asm_value = body->y;
    kq2d_asm_operand = body->vy;
    kq2d_add_s16_asm();
    body->y = kq2d_asm_value;
}

// Delegate strict rectangle overlap to the fixed-point utility; edge-only
// contact is false. Coordinate-plus-size sums must remain representable as s16.
u8 kq2d_rect_intersect(KQRect a, KQRect b)
{
    return kq_rect_intersect(a, b);
}

// Delegate the half-open point test: include left/top and exclude right/bottom.
// Coordinate-plus-size sums must remain representable as s16.
u8 kq2d_point_in_rect(s16 x, s16 y, KQRect r)
{
    return kq_point_in_rect(x, y, r);
}

// Resolve one pair with axis-aligned positional correction and simple velocity response.
// Separate overlapping boxes on the smaller-penetration axis; ties use Y.
// Split displacement by inverse mass. Two dynamic bodies receive their average
// axis velocity; a body against a static obstacle loses that axis velocity.
// Restitution is unused. Inputs and intermediate products must fit 16-bit arithmetic.
void kq2d__resolve_pair(KQBody2D* a, KQBody2D* b)
{
    if (kq2d_overlap_aabb(a, b) == 0) return;

    s16 dx = (s16)(b->x - a->x);
    s16 dy = (s16)(b->y - a->y);

    s16 adx = kq2d__abs_s16(dx);
    s16 ady = kq2d__abs_s16(dy);

    s16 px = (s16)((a->half_x + b->half_x) - adx);
    s16 py = (s16)((a->half_y + b->half_y) - ady);

    if (px <= 0 || py <= 0) return;

    s16 inv_a = a->inv_mass_q8;
    s16 inv_b = b->inv_mass_q8;
    s16 inv_sum = (s16)(inv_a + inv_b);

    // Two static bodies are not solvable.
    if (inv_sum <= 0) return;

    // Pick the axis with smaller penetration.
    u8 use_x = (u8)((px < py) ? 1 : 0);
    s16 pen = use_x ? px : py;

    s16 move_a = (s16)((pen * inv_a) / inv_sum);
    s16 move_b = (s16)(pen - move_a);

    if (use_x != 0)
    {
        s16 nx = (dx >= 0) ? 1 : (s16)-1;

        if (inv_a > 0) a->x = (s16)(a->x - (s16)(nx * move_a));
        if (inv_b > 0) b->x = (s16)(b->x + (s16)(nx * move_b));

        if (inv_a > 0 && inv_b > 0)
        {
            s16 mid = (s16)((a->vx + b->vx) >> 1);
            a->vx = mid;
            b->vx = mid;
        }
        else if (inv_a <= 0 && inv_b > 0)
        {
            b->vx = 0;
        }
        else if (inv_b <= 0 && inv_a > 0)
        {
            a->vx = 0;
        }
    }
    else
    {
        s16 ny = (dy >= 0) ? 1 : (s16)-1;

        if (inv_a > 0) a->y = (s16)(a->y - (s16)(ny * move_a));
        if (inv_b > 0) b->y = (s16)(b->y + (s16)(ny * move_b));

        if (inv_a > 0 && inv_b > 0)
        {
            s16 mid = (s16)((a->vy + b->vy) >> 1);
            a->vy = mid;
            b->vy = mid;
        }
        else if (inv_a <= 0 && inv_b > 0)
        {
            b->vy = 0;
        }
        else if (inv_b <= 0 && inv_a > 0)
        {
            a->vy = 0;
        }
    }
}

// Advance active dynamic bodies, then visit each active unordered pair for
// the configured contact passes (zero means one). Static bodies still participate
// as obstacles. The world borrows its array and shares non-reentrant ASM scratch.
// This is discrete integration; fast bodies can pass through thin obstacles.
void kq2d_step(KQWorld2D* world)
{
    if (world == 0) return;
    if (world->bodies == 0) return;

    KQBody2D* bodies = world->bodies;
    u8 count = world->body_count;

    // Integrate dynamic bodies.
    u8 i;
    for (i = 0; i < count; i++)
    {
        KQBody2D* b = &bodies[(__safe_index u8)i];
        if (b->active == 0) continue;
        if (b->inv_mass_q8 <= 0) continue;

        kq2d_asm_value = b->vx;
        kq2d_asm_operand = world->gravity_x;
        kq2d_add_s16_asm();
        kq2d_asm_limit = world->max_speed;
        kq2d_clamp_abs_s16_asm();
        b->vx = kq2d_asm_value;

        kq2d_asm_value = b->vy;
        kq2d_asm_operand = world->gravity_y;
        kq2d_add_s16_asm();
        kq2d_asm_limit = world->max_speed;
        kq2d_clamp_abs_s16_asm();
        b->vy = kq2d_asm_value;

        kq2d_asm_value = b->x;
        kq2d_asm_operand = b->vx;
        kq2d_add_s16_asm();
        b->x = kq2d_asm_value;

        kq2d_asm_value = b->y;
        kq2d_asm_operand = b->vy;
        kq2d_add_s16_asm();
        b->y = kq2d_asm_value;
    }

    // Solve contacts with a small fixed iteration count.
    u8 iterations = world->solver_iterations;
    if (iterations == 0) iterations = 1;

    u8 it;
    for (it = 0; it < iterations; it++)
    {
        for (i = 0; i < count; i++)
        {
            KQBody2D* a = &bodies[(__safe_index u8)i];
            if (a->active == 0) continue;

            u8 j;
            for (j = (u8)(i + 1); j < count; j++)
            {
                KQBody2D* b = &bodies[(__safe_index u8)j];
                if (b->active == 0) continue;
                kq2d__resolve_pair(a, b);
            }
        }
    }
}

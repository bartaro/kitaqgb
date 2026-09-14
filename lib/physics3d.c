#include "physics3d.h"

// Place this implementation in ROM bank 2. Callers must arrange compatible
// banked calls; these functions also share scratch and must not be interrupted
// by another invocation of the same physics helpers.
#pragma bank 2

s16 kq3d_asm_value;
s16 kq3d_asm_operand;
s16 kq3d_asm_limit;
s16 kq3d_asm_tmp;

// Add the shared operand to the shared value using low-byte carry.
// The 16-bit result wraps; this is not saturating addition.
void kq3d_add_s16_asm()
{
    __asm {
        LD_HL_IMM kq3d_asm_value
        LD_A_HL
        LD_B_A
        INC_HL
        LD_A_HL
        LD_C_A

        LD_HL_IMM kq3d_asm_operand
        LD_A_HL
        ADD_B
        LD_B_A
        INC_HL
        LD_A_HL
        ADC_C
        LD_C_A

        LD_HL_IMM kq3d_asm_value
        LD_A_B
        LDI_HL_A
        LD_A_C
        LD_HL_A
        RET
    }
}

// Negate a negative shared value in place. The minimum signed value remains
// 0x8000 and cannot be used as a positive s16 magnitude.
void kq3d_abs_s16_asm()
{
    __asm {
        LD_HL_IMM kq3d_asm_value
        INC_HL
        LD_A_HL
        AND_IMM 0x80
        JP_Z kq3dabs_ret

        LD_HL_IMM kq3d_asm_value
        LD_A_HL
        CPL
        ADD_A_IMM 1
        LD_HL_A
        INC_HL
        LD_A_HL
        CPL
        ADC_IMM 0
        LD_HL_A

kq3dabs_ret:
        RET
    }
}

// Clamp the shared value to +/- the shared nonnegative limit by comparing
// magnitudes high-byte first. The negative path uses shared temporary storage.
void kq3d_clamp_abs_s16_asm()
{
    __asm {
        LD_HL_IMM kq3d_asm_value
        INC_HL
        LD_A_HL
        AND_IMM 0x80
        JP_NZ kq3dcl_neg

kq3dcl_pos:
        LD_HL_IMM kq3d_asm_value
        INC_HL
        LD_A_HL
        LD_B_A
        LD_HL_IMM kq3d_asm_limit
        INC_HL
        LD_A_HL
        LD_C_A
        LD_A_B
        CP_C
        JP_C kq3dcl_done
        JP_Z kq3dcl_pos_low
        JP kq3dcl_set_pos

kq3dcl_pos_low:
        LD_HL_IMM kq3d_asm_value
        LD_A_HL
        LD_B_A
        LD_HL_IMM kq3d_asm_limit
        LD_A_HL
        LD_C_A
        LD_A_B
        CP_C
        JP_C kq3dcl_done
        JP_Z kq3dcl_done

kq3dcl_set_pos:
        LD_HL_IMM kq3d_asm_limit
        LD_A_HL
        LD_B_A
        INC_HL
        LD_A_HL
        LD_C_A
        LD_HL_IMM kq3d_asm_value
        LD_A_B
        LDI_HL_A
        LD_A_C
        LD_HL_A
        JP kq3dcl_done

kq3dcl_neg:
        LD_HL_IMM kq3d_asm_value
        LD_A_HL
        LD_B_A
        INC_HL
        LD_A_HL
        LD_C_A
        LD_HL_IMM kq3d_asm_tmp
        LD_A_B
        LDI_HL_A
        LD_A_C
        LD_HL_A

        LD_HL_IMM kq3d_asm_tmp
        LD_A_HL
        CPL
        ADD_A_IMM 1
        LD_HL_A
        INC_HL
        LD_A_HL
        CPL
        ADC_IMM 0
        LD_HL_A

        LD_HL_IMM kq3d_asm_tmp
        INC_HL
        LD_A_HL
        LD_B_A
        LD_HL_IMM kq3d_asm_limit
        INC_HL
        LD_A_HL
        LD_C_A
        LD_A_B
        CP_C
        JP_C kq3dcl_done
        JP_Z kq3dcl_neg_low
        JP kq3dcl_set_neg

kq3dcl_neg_low:
        LD_HL_IMM kq3d_asm_tmp
        LD_A_HL
        LD_B_A
        LD_HL_IMM kq3d_asm_limit
        LD_A_HL
        LD_C_A
        LD_A_B
        CP_C
        JP_C kq3dcl_done
        JP_Z kq3dcl_done

kq3dcl_set_neg:
        LD_HL_IMM kq3d_asm_limit
        LD_A_HL
        LD_B_A
        INC_HL
        LD_A_HL
        LD_C_A
        LD_HL_IMM kq3d_asm_value
        LD_A_B
        LDI_HL_A
        LD_A_C
        LD_HL_A

        LD_HL_IMM kq3d_asm_value
        LD_A_HL
        CPL
        ADD_A_IMM 1
        LD_HL_A
        INC_HL
        LD_A_HL
        CPL
        ADC_IMM 0
        LD_HL_A

kq3dcl_done:
        RET
    }
}

// Return a magnitude through shared assembly scratch. Exclude -32768 when
// a positive result is required; this helper is not reentrant.
s16 kq3d__abs_s16(s16 v)
{
    kq3d_asm_value = v;
    kq3d_abs_s16_asm();
    return kq3d_asm_value;
}

// Keep the largest incoming impact seen since the caller last cleared it.
// This does not inspect break_speed, change flags, or deactivate the body.
void kq3d__record_impact(KQBody3D* body, s16 impact)
{
    if (impact > body->last_impact_speed) body->last_impact_speed = impact;
}

// For closing motion on the selected axis, record impact on both bodies,
// then apply an inverse-mass-weighted impulse using averaged restitution clamped
// to 0..256. Axis 0 is X, 1 is Y, all others are Z; normal must be +/-1.
// Intermediate products are signed 16-bit and must remain representable.
void kq3d__bounce_axis(KQBody3D* a, KQBody3D* b, u8 axis, s16 normal)
{
    s16 va;
    s16 vb;
    s16 rel;
    s16 impact;
    s16 inv_a;
    s16 inv_b;
    s16 inv_sum;
    s16 restitution;
    s16 push;
    s16 dv_a;
    s16 dv_b;

    if (axis == 0)
    {
        va = a->vx;
        vb = b->vx;
    }
    else if (axis == 1)
    {
        va = a->vy;
        vb = b->vy;
    }
    else
    {
        va = a->vz;
        vb = b->vz;
    }

    rel = (s16)((vb - va) * normal);
    if (rel >= 0) return;

    impact = kq3d__abs_s16(rel);
    kq3d__record_impact(a, impact);
    kq3d__record_impact(b, impact);
    if (a->active == 0) return;
    if (b->active == 0) return;

    inv_a = a->inv_mass_q8;
    inv_b = b->inv_mass_q8;
    inv_sum = (s16)(inv_a + inv_b);
    if (inv_sum <= 0) return;

    restitution = (s16)((a->restitution_q8 + b->restitution_q8) >> 1);
    if (restitution < 0) restitution = 0;
    if (restitution > 256) restitution = 256;

    push = (s16)((impact * (s16)(256 + restitution)) >> 8);
    if (push <= 0) push = 1;
    dv_a = (s16)((push * inv_a) / inv_sum);
    dv_b = (s16)(push - dv_a);

    if (axis == 0)
    {
        if (inv_a > 0) a->vx = (s16)(a->vx - (s16)(normal * dv_a));
        if (inv_b > 0) b->vx = (s16)(b->vx + (s16)(normal * dv_b));
    }
    else if (axis == 1)
    {
        if (inv_a > 0) a->vy = (s16)(a->vy - (s16)(normal * dv_a));
        if (inv_b > 0) b->vy = (s16)(b->vy + (s16)(normal * dv_b));
    }
    else
    {
        if (inv_a > 0) a->vz = (s16)(a->vz - (s16)(normal * dv_a));
        if (inv_b > 0) b->vz = (s16)(b->vz + (s16)(normal * dv_b));
    }
}

// Borrow the body array without initializing it. Set gravity (0,24,0), four
// contact passes and a per-axis speed limit of 512. Null world is ignored;
// the caller retains ownership and must keep the array alive.
void kq3d_world_init(KQWorld3D* world, KQBody3D* bodies, u8 body_count)
{
    if (world == 0) return;

    world->bodies = bodies;
    world->body_count = body_count;
    world->gravity_x = 0;
    world->gravity_y = 24;
    world->gravity_z = 0;
    world->solver_iterations = 4;
    world->max_speed = 512;
}

// Store the requested mass; nonpositive mass makes the body static.
// For positive mass compute inverse weight as max(1,32767/max(mass,64)).
// The stored mass retains the original input; this is a relative solver weight,
// not an exact Q8 reciprocal. Null body is ignored.
void kq3d_body_set_mass(KQBody3D* body, s16 mass_q8)
{
    if (body == 0) return;

    body->mass_q8 = mass_q8;
    if (mass_q8 <= 0)
    {
        body->inv_mass_q8 = 0;
        return;
    }

    if (mass_q8 < 64) mass_q8 = 64;
    body->inv_mass_q8 = (s16)(32767 / mass_q8);
    if (body->inv_mass_q8 <= 0) body->inv_mass_q8 = 1;
}

// Initialize an active centered box with zero velocity/acceleration,
// restitution 224, cleared impact/flags and mass set through the weight helper.
// The caller supplies valid half extents. Null body is ignored.
void kq3d_body_init(KQBody3D* body, s16 x, s16 y, s16 z, s16 half_x, s16 half_y, s16 half_z, s16 mass_q8)
{
    if (body == 0) return;

    body->x = x;
    body->y = y;
    body->z = z;
    body->vx = 0;
    body->vy = 0;
    body->vz = 0;
    body->ax = 0;
    body->ay = 0;
    body->az = 0;
    body->half_x = half_x;
    body->half_y = half_y;
    body->half_z = half_z;
    body->restitution_q8 = 224;
    body->break_speed = 0;
    body->last_impact_speed = 0;
    body->active = 1;
    body->flags = 0;
    kq3d_body_set_mass(body, mass_q8);
}

// Test strict overlap on all three axes; face-only contact returns false.
// Valid pointers are required and flags are ignored. Center differences and
// extent sums must fit the signed 16-bit magnitude calculations.
u8 kq3d_overlap_aabb(const KQBody3D* a, const KQBody3D* b)
{
    s16 dx = kq3d__abs_s16((s16)(b->x - a->x));
    s16 dy = kq3d__abs_s16((s16)(b->y - a->y));
    s16 dz = kq3d__abs_s16((s16)(b->z - a->z));

    s16 sx = (s16)(a->half_x + b->half_x);
    s16 sy = (s16)(a->half_y + b->half_y);
    s16 sz = (s16)(a->half_z + b->half_z);

    if (dx >= sx) return 0;
    if (dy >= sy) return 0;
    if (dz >= sz) return 0;
    return 1;
}

// Resolve one pair with axis-aligned positional correction and mass-weighted bounce.
// Separate overlapping boxes on the least-penetrating axis, preferring X
// then Y then Z on ties, and apply an axis impulse for closing velocity.
// Position correction uses inverse-mass weights; two static bodies are skipped.
// Intermediate products must fit 16 bits; break flags are not set here.
void kq3d__resolve_pair(KQBody3D* a, KQBody3D* b)
{
    if (kq3d_overlap_aabb(a, b) == 0) return;

    s16 dx = (s16)(b->x - a->x);
    s16 dy = (s16)(b->y - a->y);
    s16 dz = (s16)(b->z - a->z);

    s16 adx = kq3d__abs_s16(dx);
    s16 ady = kq3d__abs_s16(dy);
    s16 adz = kq3d__abs_s16(dz);

    s16 px = (s16)((a->half_x + b->half_x) - adx);
    s16 py = (s16)((a->half_y + b->half_y) - ady);
    s16 pz = (s16)((a->half_z + b->half_z) - adz);

    if (px <= 0 || py <= 0 || pz <= 0) return;

    s16 inv_a = a->inv_mass_q8;
    s16 inv_b = b->inv_mass_q8;
    s16 inv_sum = (s16)(inv_a + inv_b);
    if (inv_sum <= 0) return;

    u8 axis = 0; // 0:x, 1:y, 2:z
    s16 pen = px;
    if (py < pen) { pen = py; axis = 1; }
    if (pz < pen) { pen = pz; axis = 2; }

    s16 move_a = (s16)((pen * inv_a) / inv_sum);
    s16 move_b = (s16)(pen - move_a);

    if (axis == 0)
    {
        s16 nx = (dx >= 0) ? 1 : (s16)-1;
        if (inv_a > 0) a->x = (s16)(a->x - (s16)(nx * move_a));
        if (inv_b > 0) b->x = (s16)(b->x + (s16)(nx * move_b));
        kq3d__bounce_axis(a, b, 0, nx);
    }
    else if (axis == 1)
    {
        s16 ny = (dy >= 0) ? 1 : (s16)-1;
        if (inv_a > 0) a->y = (s16)(a->y - (s16)(ny * move_a));
        if (inv_b > 0) b->y = (s16)(b->y + (s16)(ny * move_b));
        kq3d__bounce_axis(a, b, 1, ny);
    }
    else
    {
        s16 nz = (dz >= 0) ? 1 : (s16)-1;
        if (inv_a > 0) a->z = (s16)(a->z - (s16)(nz * move_a));
        if (inv_b > 0) b->z = (s16)(b->z + (s16)(nz * move_b));
        kq3d__bounce_axis(a, b, 2, nz);
    }
}

// Reset impact for one active dynamic body, add gravity and persistent body
// acceleration, clamp velocity per axis and advance position. Static/inactive
// bodies retain impact state. Null inputs are ignored; no contacts are solved.
// Use a nonnegative speed limit and avoid overflow before clamping.
void kq3d_integrate_body(KQWorld3D* world, KQBody3D* body)
{
    if (world == 0) return;
    if (body == 0) return;
    if (body->active == 0) return;
    if (body->inv_mass_q8 <= 0) return;

    body->last_impact_speed = 0;
    kq3d_asm_value = body->vx;
    kq3d_asm_operand = world->gravity_x;
    kq3d_add_s16_asm();
    kq3d_asm_operand = body->ax;
    kq3d_add_s16_asm();
    kq3d_asm_limit = world->max_speed;
    kq3d_clamp_abs_s16_asm();
    body->vx = kq3d_asm_value;

    kq3d_asm_value = body->vy;
    kq3d_asm_operand = world->gravity_y;
    kq3d_add_s16_asm();
    kq3d_asm_operand = body->ay;
    kq3d_add_s16_asm();
    kq3d_asm_limit = world->max_speed;
    kq3d_clamp_abs_s16_asm();
    body->vy = kq3d_asm_value;

    kq3d_asm_value = body->vz;
    kq3d_asm_operand = world->gravity_z;
    kq3d_add_s16_asm();
    kq3d_asm_operand = body->az;
    kq3d_add_s16_asm();
    kq3d_asm_limit = world->max_speed;
    kq3d_clamp_abs_s16_asm();
    body->vz = kq3d_asm_value;

    kq3d_asm_value = body->x;
    kq3d_asm_operand = body->vx;
    kq3d_add_s16_asm();
    body->x = kq3d_asm_value;

    kq3d_asm_value = body->y;
    kq3d_asm_operand = body->vy;
    kq3d_add_s16_asm();
    body->y = kq3d_asm_value;

    kq3d_asm_value = body->z;
    kq3d_asm_operand = body->vz;
    kq3d_add_s16_asm();
    body->z = kq3d_asm_value;
}

// Integrate active dynamic bodies and reset their impact values, then solve
// each active unordered pair for the configured passes (zero means one).
// Static bodies retain previous impact maxima until the game clears them.
// Acceleration persists; flags and break thresholds remain game-owned.
// This discrete solver shares non-reentrant ASM scratch and can miss fast crossings.
void kq3d_step(KQWorld3D* world)
{
    if (world == 0) return;
    if (world->bodies == 0) return;

    KQBody3D* bodies = world->bodies;
    u8 count = world->body_count;

    u8 i;
    for (i = 0; i < count; i++)
    {
        KQBody3D* b = &bodies[(__safe_index u8)i];
        if (b->active == 0) continue;
        if (b->inv_mass_q8 <= 0) continue;

        b->last_impact_speed = 0;
        kq3d_asm_value = b->vx;
        kq3d_asm_operand = world->gravity_x;
        kq3d_add_s16_asm();
        kq3d_asm_operand = b->ax;
        kq3d_add_s16_asm();
        kq3d_asm_limit = world->max_speed;
        kq3d_clamp_abs_s16_asm();
        b->vx = kq3d_asm_value;

        kq3d_asm_value = b->vy;
        kq3d_asm_operand = world->gravity_y;
        kq3d_add_s16_asm();
        kq3d_asm_operand = b->ay;
        kq3d_add_s16_asm();
        kq3d_asm_limit = world->max_speed;
        kq3d_clamp_abs_s16_asm();
        b->vy = kq3d_asm_value;

        kq3d_asm_value = b->vz;
        kq3d_asm_operand = world->gravity_z;
        kq3d_add_s16_asm();
        kq3d_asm_operand = b->az;
        kq3d_add_s16_asm();
        kq3d_asm_limit = world->max_speed;
        kq3d_clamp_abs_s16_asm();
        b->vz = kq3d_asm_value;

        kq3d_asm_value = b->x;
        kq3d_asm_operand = b->vx;
        kq3d_add_s16_asm();
        b->x = kq3d_asm_value;

        kq3d_asm_value = b->y;
        kq3d_asm_operand = b->vy;
        kq3d_add_s16_asm();
        b->y = kq3d_asm_value;

        kq3d_asm_value = b->z;
        kq3d_asm_operand = b->vz;
        kq3d_add_s16_asm();
        b->z = kq3d_asm_value;
    }

    u8 iterations = world->solver_iterations;
    if (iterations == 0) iterations = 1;

    u8 it;
    for (it = 0; it < iterations; it++)
    {
        for (i = 0; i < count; i++)
        {
            KQBody3D* a = &bodies[(__safe_index u8)i];
            if (a->active == 0) continue;

            u8 j;
            for (j = (u8)(i + 1); j < count; j++)
            {
                KQBody3D* b = &bodies[(__safe_index u8)j];
                if (b->active == 0) continue;
                kq3d__resolve_pair(a, b);
            }
        }
    }
}

// Sum three signed intrinsic products, each shifted by eight bits before
// addition. Coefficients are signed bytes scaled by 1/256. The 16-bit sum is
// not saturated, so keep products and their sum within the intended range.
s16 kq3d_dot_q8_8(s16 x, s16 y, s16 z, s8 ax, s8 ay, s8 az)
{
    return __sdot3_q8_8(x, y, z, ax, ay, az);
}

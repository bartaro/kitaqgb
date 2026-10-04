#include "physics3d.h"

#pragma bank 2

s16 kq3d_asm_value;
s16 kq3d_asm_operand;
s16 kq3d_asm_limit;
s16 kq3d_asm_tmp;

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

s16 kq3d__abs_s16(s16 v)
{
    kq3d_asm_value = v;
    kq3d_abs_s16_asm();
    return kq3d_asm_value;
}

void kq3d__record_impact(KQBody3D* body, s16 impact)
{
    if (impact > body->last_impact_speed) body->last_impact_speed = impact;
}

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

// Exact sphere-versus-box overlap after a cheap AABB broad-phase check.
// Sphere radius is half_x; other half extents are ignored for flagged spheres.
u8 kq3d_overlap_sphere_aabb(const KQBody3D* sphere, const KQBody3D* box)
{
    s16 dx;
    s16 dy;
    s16 dz;
    s16 radius;
    s16 closest_x;
    s16 closest_y;
    s16 closest_z;
    u16 distance_sq;
    u16 radius_sq;
    u16 distance;

    if (sphere == 0 || box == 0) return 0;
    radius = sphere->half_x;
    if (radius <= 0) return 0;
    // Inclusive broad phase: tangent spheres still need an impulse when they
    // are moving into a paddle face.
    if (kq3d__abs_s16((s16)(sphere->x - box->x)) > (s16)(radius + box->half_x)) return 0;
    if (kq3d__abs_s16((s16)(sphere->y - box->y)) > (s16)(radius + box->half_y)) return 0;
    if (kq3d__abs_s16((s16)(sphere->z - box->z)) > (s16)(radius + box->half_z)) return 0;

    dx = (s16)(sphere->x - box->x);
    dy = (s16)(sphere->y - box->y);
    dz = (s16)(sphere->z - box->z);
    closest_x = dx;
    closest_y = dy;
    closest_z = dz;
    if (closest_x < (s16)(0 - box->half_x)) closest_x = (s16)(0 - box->half_x);
    else if (closest_x > box->half_x) closest_x = box->half_x;
    if (closest_y < (s16)(0 - box->half_y)) closest_y = (s16)(0 - box->half_y);
    else if (closest_y > box->half_y) closest_y = box->half_y;
    if (closest_z < (s16)(0 - box->half_z)) closest_z = (s16)(0 - box->half_z);
    else if (closest_z > box->half_z) closest_z = box->half_z;

    dx = (s16)(dx - closest_x);
    dy = (s16)(dy - closest_y);
    dz = (s16)(dz - closest_z);
    if (kq3d__abs_s16(dx) > radius) return 0;
    if (kq3d__abs_s16(dy) > radius) return 0;
    if (kq3d__abs_s16(dz) > radius) return 0;

    distance_sq = (u16)((u16)(dx * dx) + (u16)(dy * dy) + (u16)(dz * dz));
    radius_sq = (u16)(radius * radius);
    if (distance_sq <= radius_sq) return 1;
    distance = 0;
    while ((u16)((distance + 1) * (distance + 1)) <= distance_sq)
        distance++;
    return (u8)(distance < (u16)radius);
}

static s16 kq3d__clamp_s16(s16 value, s16 low, s16 high)
{
    if (value < low) return low;
    if (value > high) return high;
    return value;
}

static void kq3d__bounce_normal(KQBody3D* a, KQBody3D* b, s16 nx, s16 ny, s16 nz)
{
    s16 rel_x = (s16)(b->vx - a->vx);
    s16 rel_y = (s16)(b->vy - a->vy);
    s16 rel_z = (s16)(b->vz - a->vz);
    s16 rel = (s16)(((rel_x * nx) + (rel_y * ny) + (rel_z * nz)) >> 8);
    s16 impact;
    s16 inv_a;
    s16 inv_b;
    s16 inv_sum;
    s16 restitution;
    s16 push;
    s16 dv_a;
    s16 dv_b;

    if (rel >= 0) return;
    impact = (s16)(0 - rel);
    kq3d__record_impact(a, impact);
    kq3d__record_impact(b, impact);
    if (a->active == 0 || b->active == 0) return;
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
    if (inv_a > 0)
    {
        a->vx = (s16)(a->vx - (s16)((nx * dv_a) >> 8));
        a->vy = (s16)(a->vy - (s16)((ny * dv_a) >> 8));
        a->vz = (s16)(a->vz - (s16)((nz * dv_a) >> 8));
    }
    if (inv_b > 0)
    {
        b->vx = (s16)(b->vx + (s16)((nx * dv_b) >> 8));
        b->vy = (s16)(b->vy + (s16)((ny * dv_b) >> 8));
        b->vz = (s16)(b->vz + (s16)((nz * dv_b) >> 8));
    }
}

static void kq3d__resolve_sphere_box(KQBody3D* a, KQBody3D* b, KQBody3D* sphere, KQBody3D* box)
{
    s16 rx = (s16)(sphere->x - box->x);
    s16 ry = (s16)(sphere->y - box->y);
    s16 rz = (s16)(sphere->z - box->z);
    s16 qx = kq3d__clamp_s16(rx, (s16)(0 - box->half_x), box->half_x);
    s16 qy = kq3d__clamp_s16(ry, (s16)(0 - box->half_y), box->half_y);
    s16 qz = kq3d__clamp_s16(rz, (s16)(0 - box->half_z), box->half_z);
    s16 tx = (s16)(qx - rx);
    s16 ty = (s16)(qy - ry);
    s16 tz = (s16)(qz - rz);
    s16 nx;
    s16 ny;
    s16 nz;
    s16 penetration;
    s16 inv_a;
    s16 inv_b;
    s16 inv_sum;
    s16 move_a;
    s16 move_b;
    u16 distance_sq;
    u16 distance;
    u8 sphere_is_a = (u8)(sphere == a);

    if (kq3d_overlap_sphere_aabb(sphere, box) == 0) return;
    distance_sq = (u16)((u16)(tx * tx) + (u16)(ty * ty) + (u16)(tz * tz));
    if (distance_sq != 0)
    {
        distance = 0;
        while ((u16)((distance + 1) * (distance + 1)) <= distance_sq)
            distance++;
        if (distance == 0) distance = 1;
        penetration = (s16)(sphere->half_x - (s16)distance);
        nx = (s16)((tx * (s16)256) / (s16)distance);
        ny = (s16)((ty * (s16)256) / (s16)distance);
        nz = (s16)((tz * (s16)256) / (s16)distance);
    }
    else
    {
        // When the center is inside the box, separate it through the nearest
        // face. At an exact center tie, velocity selects the incoming side.
        s16 rvx = (s16)(sphere->vx - box->vx);
        s16 rvy = (s16)(sphere->vy - box->vy);
        s16 rvz = (s16)(sphere->vz - box->vz);
        s16 gap_x = (s16)(box->half_x - kq3d__abs_s16(rx));
        s16 gap_y = (s16)(box->half_y - kq3d__abs_s16(ry));
        s16 gap_z = (s16)(box->half_z - kq3d__abs_s16(rz));

        nx = 0; ny = 0; nz = 0;
        if (gap_x <= gap_y && gap_x <= gap_z)
        {
            nx = (rx > 0 || (rx == 0 && rvx < 0)) ? (s16)-256 : (s16)256;
            penetration = (s16)(sphere->half_x + gap_x);
        }
        else if (gap_y <= gap_z)
        {
            ny = (ry > 0 || (ry == 0 && rvy < 0)) ? (s16)-256 : (s16)256;
            penetration = (s16)(sphere->half_x + gap_y);
        }
        else
        {
            nz = (rz > 0 || (rz == 0 && rvz < 0)) ? (s16)-256 : (s16)256;
            penetration = (s16)(sphere->half_x + gap_z);
        }
    }

    if (sphere_is_a == 0)
    {
        nx = (s16)(0 - nx);
        ny = (s16)(0 - ny);
        nz = (s16)(0 - nz);
    }
    inv_a = a->inv_mass_q8;
    inv_b = b->inv_mass_q8;
    inv_sum = (s16)(inv_a + inv_b);
    if (inv_sum <= 0) return;
    if (penetration < 0) penetration = 0;
    move_a = (s16)((penetration * inv_a) / inv_sum);
    move_b = (s16)(penetration - move_a);
    if (inv_a > 0)
    {
        a->x = (s16)(a->x - (s16)((nx * move_a) >> 8));
        a->y = (s16)(a->y - (s16)((ny * move_a) >> 8));
        a->z = (s16)(a->z - (s16)((nz * move_a) >> 8));
    }
    if (inv_b > 0)
    {
        b->x = (s16)(b->x + (s16)((nx * move_b) >> 8));
        b->y = (s16)(b->y + (s16)((ny * move_b) >> 8));
        b->z = (s16)(b->z + (s16)((nz * move_b) >> 8));
    }
    kq3d__bounce_normal(a, b, nx, ny, nz);
}

// Resolve one pair with axis-aligned positional correction and mass-weighted bounce.
void kq3d__resolve_pair(KQBody3D* a, KQBody3D* b)
{
    if ((a->flags & KQ3D_BODY_SPHERE) != 0 && (b->flags & KQ3D_BODY_SPHERE) == 0)
    {
        kq3d__resolve_sphere_box(a, b, a, b);
        return;
    }
    if ((b->flags & KQ3D_BODY_SPHERE) != 0 && (a->flags & KQ3D_BODY_SPHERE) == 0)
    {
        kq3d__resolve_sphere_box(a, b, b, a);
        return;
    }
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

static void kq3d__integrate_body(KQWorld3D* world, KQBody3D* body, u8 allow_static)
{
    if (world == 0) return;
    if (body == 0) return;
    if (body->active == 0) return;
    if (body->inv_mass_q8 <= 0 && allow_static == 0) return;

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

void kq3d_integrate_body(KQWorld3D* world, KQBody3D* body)
{
    kq3d__integrate_body(world, body, 0);
}

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
        u8 kinematic = (u8)(b->flags & KQ3D_BODY_KINEMATIC);
        if (b->active == 0) continue;
        if (b->inv_mass_q8 <= 0 && kinematic == 0) continue;

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

        // Kinematic bodies only move on their controlled X/Y axes. Their
        // inverse mass remains zero, so contacts never push them around.
        if (kinematic != 0)
        {
            kq3d_asm_value = b->x;
            kq3d_asm_operand = b->vx;
            kq3d_add_s16_asm();
            b->x = kq3d_asm_value;
            kq3d_asm_value = b->y;
            kq3d_asm_operand = b->vy;
            kq3d_add_s16_asm();
            b->y = kq3d_asm_value;
            continue;
        }

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
                // Two immovable bodies cannot receive a solver correction.
                // Skip their AABB test because resolve_pair would return once
                // it finds the inverse-mass sum is zero.
                if (a->inv_mass_q8 <= 0 && b->inv_mass_q8 <= 0) continue;
                kq3d__resolve_pair(a, b);
            }
        }
    }
}

s16 kq3d_dot_q8_8(s16 x, s16 y, s16 z, s8 ax, s8 ay, s8 az)
{
    return __sdot3_q8_8(x, y, z, ax, ay, az);
}

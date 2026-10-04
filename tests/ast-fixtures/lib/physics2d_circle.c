#include "physics2d_circle.h"

u16 __mul16x8(u16 a, u8 b);

s16 kq2dc__abs_s16(s16 v)
{
    if (v < 0) return (s16)(0 - v);
    return v;
}

s16 kq2dc__clamp_abs(s16 v, s16 limit)
{
    if (v > limit) return limit;
    if (v < (s16)(0 - limit)) return (s16)(0 - limit);
    return v;
}

s16 kq2dc__halve_keep_sign(s16 v)
{
    if (v > 0) return (s16)((v + 1) >> 1);
    if (v < 0) return (s16)(0 - (((0 - v) + 1) >> 1));
    return 0;
}

u16 kq2dc__isqrt_u16(u16 value)
{
    u16 result = 0;
    u16 bit = 16384;

    while (bit > value) bit = (u16)(bit >> 2);

    while (bit != 0)
    {
        u16 test = (u16)(result + bit);
        if (value >= test)
        {
            value = (u16)(value - test);
            result = (u16)((result >> 1) + bit);
        }
        else
        {
            result = (u16)(result >> 1);
        }
        bit = (u16)(bit >> 2);
    }

    return result;
}

u8 kq2dc__reduce_vector(s16* x, s16* y)
{
    u8 shift = 0;

    while (kq2dc__abs_s16(*x) > 255 || kq2dc__abs_s16(*y) > 255)
    {
        *x = kq2dc__halve_keep_sign(*x);
        *y = kq2dc__halve_keep_sign(*y);
        shift = (u8)(shift + 1);
    }

    return shift;
}

u16 kq2dc__scale_down_positive(s16 value, u8 shift)
{
    s16 v = value;

    while (shift != 0)
    {
        v = (s16)((v + 1) >> 1);
        shift = (u8)(shift - 1);
    }

    if (v <= 0) return 0;
    return (u16)v;
}

s8 kq2dc__unit_q1_7(s16 num, u16 den)
{
    s16 scaled;

    if (den == 0) return 0;

    scaled = (s16)((num * 127) / (s16)den);
    if (scaled > 127) scaled = 127;
    if (scaled < -127) scaled = -127;
    return (s8)scaled;
}

u8 kq2dc__ratio_q8(s16 part, s16 total)
{
    s16 ratio;

    if (part <= 0 || total <= 0) return 0;
    if (part >= total) return 255;

    ratio = (s16)((part * 255) / total);
    if (ratio < 0) ratio = 0;
    if (ratio > 255) ratio = 255;
    return (u8)ratio;
}

u8 kq2dc__avg_u8(u8 a, u8 b)
{
    return (u8)(((u16)a + (u16)b) >> 1);
}

s16 kq2dc__mul_q8(s16 value, u8 coeff_q8)
{
    if (value < 0) return (s16)(0 - (s16)__mul16x8((u16)(0 - value), coeff_q8));
    return (s16)__mul16x8((u16)value, coeff_q8);
}

s16 kq2dc__mul_q1_7(s16 value, s8 coeff_q1_7)
{
    return (s16)__smul16x8_q1_7(value, coeff_q1_7);
}

s16 kq2dc__dot_q1_7(s16 x, s16 y, s8 ax, s8 ay)
{
    return (s16)__sdot2_q1_7(x, y, ax, ay);
}

void kq2dc__add_along(s16* x, s16* y, s16 amount, s8 ax, s8 ay)
{
    *x = (s16)(*x + kq2dc__mul_q1_7(amount, ax));
    *y = (s16)(*y + kq2dc__mul_q1_7(amount, ay));
}

void kq2dc__resolve_bounds(KQCircleWorld2D* world, KQCircleBody2D* body)
{
    s16 left;
    s16 right;
    s16 top;
    s16 bottom;
    u8 tangent_keep_q8;

    if (world->use_bounds == 0) return;
    if (body->radius <= 0) return;

    left = (s16)(world->min_x + body->radius);
    right = (s16)(world->max_x - body->radius);
    top = (s16)(world->min_y + body->radius);
    bottom = (s16)(world->max_y - body->radius);
    tangent_keep_q8 = (u8)(255 - world->wall_friction_q8);

    if (body->x < left)
    {
        body->x = left;
        if (body->vx < 0)
        {
            body->vx = kq2dc__mul_q8((s16)(0 - body->vx), world->wall_restitution_q8);
            body->vy = kq2dc__mul_q8(body->vy, tangent_keep_q8);
        }
    }
    else if (body->x > right)
    {
        body->x = right;
        if (body->vx > 0)
        {
            body->vx = (s16)(0 - kq2dc__mul_q8(body->vx, world->wall_restitution_q8));
            body->vy = kq2dc__mul_q8(body->vy, tangent_keep_q8);
        }
    }

    if (body->y < top)
    {
        body->y = top;
        if (body->vy < 0)
        {
            body->vy = kq2dc__mul_q8((s16)(0 - body->vy), world->wall_restitution_q8);
            body->vx = kq2dc__mul_q8(body->vx, tangent_keep_q8);
        }
    }
    else if (body->y > bottom)
    {
        body->y = bottom;
        if (body->vy > 0)
        {
            body->vy = (s16)(0 - kq2dc__mul_q8(body->vy, world->wall_restitution_q8));
            body->vx = kq2dc__mul_q8(body->vx, tangent_keep_q8);
        }
    }
}

void kq2dc__resolve_pair(KQCircleBody2D* a, KQCircleBody2D* b)
{
    s16 dx;
    s16 dy;
    s16 sdx;
    s16 sdy;
    u8 shift;
    u16 adx;
    u16 ady;
    u16 dist_sq;
    u16 dist_scaled;
    s16 radius_sum;
    u16 radius_sum_scaled;
    s16 overlap;
    s16 inv_a;
    s16 inv_b;
    s16 inv_sum;
    s8 nx;
    s8 ny;
    s8 tx;
    s8 ty;
    u8 weight_a_q8;
    u8 weight_b_q8;
    s16 move_a;
    s16 move_b;
    s16 rvx;
    s16 rvy;
    s16 vn;
    s16 close_speed;
    u8 restitution_q8;
    s16 bounce_speed;
    s16 delta_a_n;
    s16 delta_b_n;
    s16 vt;
    u8 friction_q8;
    s16 friction_speed;
    s16 delta_a_t;
    s16 delta_b_t;

    if (a->active == 0 || b->active == 0) return;
    if (a->radius <= 0 || b->radius <= 0) return;

    dx = (s16)(b->x - a->x);
    dy = (s16)(b->y - a->y);
    sdx = dx;
    sdy = dy;
    shift = kq2dc__reduce_vector(&sdx, &sdy);

    adx = (u16)kq2dc__abs_s16(sdx);
    ady = (u16)kq2dc__abs_s16(sdy);
    dist_sq = (u16)((adx * adx) + (ady * ady));

    if (dist_sq == 0)
    {
        dist_scaled = 1;
        sdx = 1;
        sdy = 0;
    }
    else
    {
        dist_scaled = kq2dc__isqrt_u16(dist_sq);
        if (dist_scaled == 0) dist_scaled = 1;
    }

    radius_sum = (s16)(a->radius + b->radius);
    radius_sum_scaled = kq2dc__scale_down_positive(radius_sum, shift);

    if (dist_scaled >= radius_sum_scaled) return;

    overlap = (s16)(radius_sum_scaled - dist_scaled);
    while (shift != 0)
    {
        overlap = (s16)(overlap << 1);
        shift = (u8)(shift - 1);
    }
    if (overlap <= 0) overlap = 1;

    inv_a = a->inv_mass_q8;
    inv_b = b->inv_mass_q8;
    inv_sum = (s16)(inv_a + inv_b);
    if (inv_sum <= 0) return;

    nx = kq2dc__unit_q1_7(sdx, dist_scaled);
    ny = kq2dc__unit_q1_7(sdy, dist_scaled);
    if (nx == 0 && ny == 0) nx = 127;
    tx = (s8)(0 - ny);
    ty = nx;

    weight_a_q8 = kq2dc__ratio_q8(inv_a, inv_sum);
    weight_b_q8 = kq2dc__ratio_q8(inv_b, inv_sum);

    move_a = kq2dc__mul_q8(overlap, weight_a_q8);
    move_b = kq2dc__mul_q8(overlap, weight_b_q8);
    if (inv_a > 0) kq2dc__add_along(&a->x, &a->y, (s16)(0 - move_a), nx, ny);
    if (inv_b > 0) kq2dc__add_along(&b->x, &b->y, move_b, nx, ny);

    rvx = (s16)(b->vx - a->vx);
    rvy = (s16)(b->vy - a->vy);
    vn = kq2dc__dot_q1_7(rvx, rvy, nx, ny);

    if (vn < 0)
    {
        close_speed = (s16)(0 - vn);
        restitution_q8 = kq2dc__avg_u8(a->restitution_q8, b->restitution_q8);
        bounce_speed = (s16)(close_speed + kq2dc__mul_q8(close_speed, restitution_q8));

        delta_a_n = kq2dc__mul_q8(bounce_speed, weight_a_q8);
        delta_b_n = kq2dc__mul_q8(bounce_speed, weight_b_q8);

        if (inv_a > 0) kq2dc__add_along(&a->vx, &a->vy, (s16)(0 - delta_a_n), nx, ny);
        if (inv_b > 0) kq2dc__add_along(&b->vx, &b->vy, delta_b_n, nx, ny);
    }

    rvx = (s16)(b->vx - a->vx);
    rvy = (s16)(b->vy - a->vy);
    vt = kq2dc__dot_q1_7(rvx, rvy, tx, ty);

    if (vt != 0)
    {
        friction_q8 = kq2dc__avg_u8(a->friction_q8, b->friction_q8);
        friction_speed = kq2dc__mul_q8(kq2dc__abs_s16(vt), friction_q8);

        delta_a_t = kq2dc__mul_q8(friction_speed, weight_a_q8);
        delta_b_t = kq2dc__mul_q8(friction_speed, weight_b_q8);

        if (vt < 0)
        {
            delta_a_t = (s16)(0 - delta_a_t);
            delta_b_t = (s16)(0 - delta_b_t);
        }

        if (inv_a > 0) kq2dc__add_along(&a->vx, &a->vy, delta_a_t, tx, ty);
        if (inv_b > 0) kq2dc__add_along(&b->vx, &b->vy, (s16)(0 - delta_b_t), tx, ty);
    }
}

void kq2dc_world_init(KQCircleWorld2D* world, KQCircleBody2D* bodies, u8 body_count)
{
    if (world == 0) return;

    world->bodies = bodies;
    world->body_count = body_count;
    world->gravity_x = 0;
    world->gravity_y = 0;
    world->solver_iterations = 4;
    world->max_speed = 768;
    world->linear_damping_q8 = 252;
    world->use_bounds = 0;
    world->min_x = 0;
    world->min_y = 0;
    world->max_x = 159;
    world->max_y = 143;
    world->wall_restitution_q8 = 240;
    world->wall_friction_q8 = 16;
}

void kq2dc_set_bounds(KQCircleWorld2D* world, s16 min_x, s16 min_y, s16 max_x, s16 max_y)
{
    if (world == 0) return;

    world->use_bounds = 1;
    world->min_x = min_x;
    world->min_y = min_y;
    world->max_x = max_x;
    world->max_y = max_y;
}

u8 kq2dc_overlap_circle(const KQCircleBody2D* a, const KQCircleBody2D* b)
{
    s16 dx;
    s16 dy;
    s16 sdx;
    s16 sdy;
    u8 shift;
    u16 adx;
    u16 ady;
    u16 dist_sq;
    u16 dist_scaled;
    s16 radius_sum;
    u16 radius_sum_scaled;

    dx = (s16)(b->x - a->x);
    dy = (s16)(b->y - a->y);
    sdx = dx;
    sdy = dy;
    shift = kq2dc__reduce_vector(&sdx, &sdy);

    adx = (u16)kq2dc__abs_s16(sdx);
    ady = (u16)kq2dc__abs_s16(sdy);
    dist_sq = (u16)((adx * adx) + (ady * ady));
    dist_scaled = kq2dc__isqrt_u16(dist_sq);

    radius_sum = (s16)(a->radius + b->radius);
    radius_sum_scaled = kq2dc__scale_down_positive(radius_sum, shift);

    if (dist_scaled >= radius_sum_scaled) return 0;
    return 1;
}

void kq2dc_step(KQCircleWorld2D* world)
{
    KQCircleBody2D* bodies;
    u8 count;
    u8 i;
    u8 iterations;
    u8 it;

    if (world == 0) return;
    if (world->bodies == 0) return;

    bodies = world->bodies;
    count = world->body_count;

    for (i = 0; i < count; i++)
    {
        KQCircleBody2D* body = &bodies[(__safe_index u8)i];
        if (body->active == 0) continue;
        if (body->inv_mass_q8 <= 0)
        {
            if (world->use_bounds != 0) kq2dc__resolve_bounds(world, body);
            continue;
        }

        body->vx = (s16)(body->vx + world->gravity_x);
        body->vy = (s16)(body->vy + world->gravity_y);
        body->vx = kq2dc__clamp_abs(body->vx, world->max_speed);
        body->vy = kq2dc__clamp_abs(body->vy, world->max_speed);

        body->x = (s16)(body->x + body->vx);
        body->y = (s16)(body->y + body->vy);

        if (world->use_bounds != 0) kq2dc__resolve_bounds(world, body);
    }

    iterations = world->solver_iterations;
    if (iterations == 0) iterations = 1;

    for (it = 0; it < iterations; it++)
    {
        for (i = 0; i < count; i++)
        {
            KQCircleBody2D* a = &bodies[(__safe_index u8)i];
            if (a->active == 0) continue;
            if (a->vx == 0 && a->vy == 0) continue;

            if (world->use_bounds != 0) kq2dc__resolve_bounds(world, a);

            {
                u8 j;
                for (j = 0; j < count; j++)
                {
                    KQCircleBody2D* b = &bodies[(__safe_index u8)j];
                    s16 radius_sum;
                    if (j == i) continue;
                    if (b->active == 0) continue;
                    if (j < i && (b->vx != 0 || b->vy != 0)) continue;
                    radius_sum = (s16)(a->radius + b->radius);
                    if (kq2dc__abs_s16((s16)(b->x - a->x)) >= radius_sum) continue;
                    if (kq2dc__abs_s16((s16)(b->y - a->y)) >= radius_sum) continue;
                    kq2dc__resolve_pair(a, b);
                }
            }
        }
    }

    for (i = 0; i < count; i++)
    {
        KQCircleBody2D* body = &bodies[(__safe_index u8)i];
        if (body->active == 0) continue;
        if (body->inv_mass_q8 <= 0) continue;

        body->vx = kq2dc__mul_q8(body->vx, world->linear_damping_q8);
        body->vy = kq2dc__mul_q8(body->vy, world->linear_damping_q8);

        if (body->vx >= -1 && body->vx <= 1) body->vx = 0;
        if (body->vy >= -1 && body->vy <= 1) body->vy = 0;

        if (world->use_bounds != 0) kq2dc__resolve_bounds(world, body);
    }
}

#pragma once

// Deterministic 2D circle physics for KITAQGB.
// Intended for top-down ball games such as billiards, air hockey, and marbles.

typedef struct {
    s16 x;
    s16 y;
    s16 vx;
    s16 vy;
    s16 radius;
    s16 inv_mass_q8;      // 0 means static body.
    u8 restitution_q8;    // 0..255, higher means bouncier.
    u8 friction_q8;       // 0..255, higher removes more tangential motion.
    u8 active;
} KQCircleBody2D;

typedef struct {
    KQCircleBody2D* bodies;
    u8 body_count;
    s16 gravity_x;
    s16 gravity_y;
    u8 solver_iterations;
    s16 max_speed;
    u8 linear_damping_q8;   // 255 is almost no damping, lower values damp faster.
    u8 use_bounds;
    s16 min_x;
    s16 min_y;
    s16 max_x;
    s16 max_y;
    u8 wall_restitution_q8;
    u8 wall_friction_q8;
} KQCircleWorld2D;

void kq2dc_world_init(KQCircleWorld2D* world, KQCircleBody2D* bodies, u8 body_count);
void kq2dc_set_bounds(KQCircleWorld2D* world, s16 min_x, s16 min_y, s16 max_x, s16 max_y);
void kq2dc_step(KQCircleWorld2D* world);
u8 kq2dc_overlap_circle(const KQCircleBody2D* a, const KQCircleBody2D* b);

#pragma once

// Deterministic 2D circle physics for KITAQGB.
// Intended for top-down ball games such as billiards, air hockey, and marbles.

// Use consistent caller-selected units for centers, radii and per-step velocity.
// The implementation uses 16-bit arithmetic: reduced squared-distance sums and
// weight products must fit their types. Reduction to byte-sized components alone
// is insufficient to guarantee this. Body storage remains caller-owned.
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

// Retain a caller-owned array without initializing bodies. Set zero gravity,
// four contact passes, per-axis limit 768 and damping 252/256; bounds start disabled.
// A null world is ignored. Keep the array alive for subsequent steps.
void kq2dc_world_init(KQCircleWorld2D* world, KQCircleBody2D* bodies, u8 body_count);
// Store and enable bounds without validating their order or fitting bodies
// immediately. Supply extents large enough for body diameters. Null is ignored.
void kq2dc_set_bounds(KQCircleWorld2D* world, s16 min_x, s16 min_y, s16 max_x, s16 max_y);
// Integrate active dynamic bodies, apply bounds, solve contacts, then damp
// velocity and snap components in -1..1 to zero. Zero solver iterations means one.
// Pairs with both velocities zero are skipped, so resting overlaps can remain.
// Bounds can reposition static bodies; null world/array inputs are ignored.
void kq2dc_step(KQCircleWorld2D* world);
// Compare a reduced integer center distance with the reduced radius sum.
// Valid pointers and positive radii are required; activity and mass are ignored.
// This approximate test requires representable differences, radius sums and
// squared-distance sum. Touching at the computed distance returns false.
u8 kq2dc_overlap_circle(const KQCircleBody2D* a, const KQCircleBody2D* b);

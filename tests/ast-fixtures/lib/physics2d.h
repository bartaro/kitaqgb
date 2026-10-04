#pragma once

#include "fixed.h"

// Multiply a signed word by a signed byte; divide by 256, truncating toward zero.
s16 __smul16x8(s16 a, s8 b);

// Lightweight deterministic 2D physics for KITAQGB.
// Units are integer pixels per frame and fixed-point inv_mass in Q8.

typedef struct {
    s16 x;
    s16 y;
    s16 vx;
    s16 vy;
    s16 half_x;
    s16 half_y;
    s16 inv_mass_q8;      // 0 means static body.
    s16 restitution_q8;   // 0..255 (reserved for future tuning).
    u8 active;
} KQBody2D;

typedef struct {
    KQBody2D* bodies;
    u8 body_count;
    s16 gravity_x;
    s16 gravity_y;
    u8 solver_iterations;
    s16 max_speed;
} KQWorld2D;

void kq2d_world_init(KQWorld2D* world, KQBody2D* bodies, u8 body_count);
void kq2d_step(KQWorld2D* world);
void kq2d_integrate_body(KQWorld2D* world, KQBody2D* body);
u8 kq2d_overlap_aabb(const KQBody2D* a, const KQBody2D* b);
void kq2d_body_init(KQBody2D* body, s16 x, s16 y, s16 half_x, s16 half_y);
void kq2d_body_set_pos(KQBody2D* body, s16 x, s16 y);
void kq2d_body_set_velocity(KQBody2D* body, s16 vx, s16 vy);
void kq2d_body_apply_gravity(KQBody2D* body, s16 gravity_x, s16 gravity_y, s16 max_speed);
void kq2d_body_apply_friction(KQBody2D* body, u8 friction_q8);
u8 kq2d_rect_intersect(KQRect a, KQRect b);
u8 kq2d_point_in_rect(s16 x, s16 y, KQRect r);

// Opt-in surface solver. Velocities/threshold/kick share the caller's units.
// Normal must be unit length in Q8; coefficients are 0..255.
typedef struct {
    s16 nx_q8;
    s16 ny_q8;
    s16 vx;
    s16 vy;
    s16 bounce_threshold;
    s16 kick;
    u8 restitution_q8;
    u8 friction_q8;
} KQSurface2D;

// Truncates toward zero, including negative values; coefficient is -256..256.
// Defining KQ2D_EXTERNAL_SCALE_Q8 while compiling physics2d.c opts out of its
// default implementation. The caller must supply this exact signed/truncating ABI.
s16 kq2d_scale_q8(s16 value, s16 coefficient);
// Conservative circular bound, with a shared Q8 scale for both components.
// Quantizes toward zero; very small limits relative to input may yield zero.
// Components and positive max_speed must be <= 16383 in magnitude.
void kq2d_body_limit_speed(KQBody2D* body, s16 max_speed);
// Kinematic surface response; returns incoming normal speed, or zero if separating.
// Caller resolves penetration separately. Relative components must fit +/-8191.
s16 kq2d_body_resolve_surface(KQBody2D* body, const KQSurface2D* surface);
// KQ2D_CACHE_SURFACE opts into direct-address surface/limiter caches in ROM bank 13.
// Default builds keep their existing layout; callers must reserve that bank.
// KQ2D_EXTERNAL_SURFACE supplies kq2d_body_resolve_surface externally with the
// same response/rounding contract. KQ2D_REGISTER_MATH additionally opts the cached
// limiter into fixed-bank register helpers: kq2d_scale_regs (DE value, BC Q8
// coefficient, HL result) and kq2d_fraction_regs (HL numerator, DE denominator,
// HL clamped Q8 fraction). These helpers may clobber AF/BC/DE and are not C APIs.
// KQ2D_EXTERNAL_REST_COMPONENT supplies kq2d__rest_component externally: round
// Q8 halves away from zero for -63..63, truncate for all other input values.
// Linear crossing time in Q8 (0..256); positive separation is outside.
// Returns 0 for initial overlap, 256 for no crossing. Gaps must fit +/-8191.
u16 kq2d_surface_toi_q8(s16 start_gap, s16 end_gap);

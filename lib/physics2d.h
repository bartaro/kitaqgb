#pragma once

// Position and velocity use caller-selected integer units with one integration
// step per call; no hidden pixel or time conversion occurs. Keep intermediate
// signed arithmetic in range. World/body storage is owned by the caller.
#include "fixed.h"

// Multiply a signed word by a signed byte; divide by 256, truncating toward zero.
s16 __smul16x8(s16 a, s8 b);

// Lightweight deterministic 2D physics for KITAQGB.
// Integer pixels per step are one possible unit convention; inv_mass is Q8.

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

// Retain the caller-owned body array without initializing its elements.
// Set gravity to (0,32), four contact iterations and a per-axis speed limit of 512.
// A null world is ignored; the array must outlive subsequent world operations.
void kq2d_world_init(KQWorld2D* world, KQBody2D* bodies, u8 body_count);
// Advance active dynamic bodies, then visit each active unordered pair for
// the configured contact passes (zero means one). Static bodies still participate
// as obstacles. The world borrows its array and shares non-reentrant ASM scratch.
// This is discrete integration; fast bodies can pass through thin obstacles.
void kq2d_step(KQWorld2D* world);
// For one active dynamic body, add gravity, clamp each velocity axis and then
// add velocity to position. Null inputs are ignored; no contacts are solved.
// Use a nonnegative speed limit and keep additions in signed 16-bit range.
void kq2d_integrate_body(KQWorld2D* world, KQBody2D* body);
// Test strict overlap of two centered boxes; touching edges return false.
// Both pointers must be valid. Active/mass flags are ignored, and coordinate
// differences and extent sums must fit the signed 16-bit calculations.
u8 kq2d_overlap_aabb(const KQBody2D* a, const KQBody2D* b);
// Initialize an active unit-inverse-mass body at the supplied center and half
// extents, with zero velocity and reserved restitution set to zero. Null is ignored.
void kq2d_body_init(KQBody2D* body, s16 x, s16 y, s16 half_x, s16 half_y);
// Replace the center without altering velocity, extents or activity. Null is ignored.
void kq2d_body_set_pos(KQBody2D* body, s16 x, s16 y);
// Replace velocity without clamping or checking activity/mass. Null is ignored.
void kq2d_body_set_velocity(KQBody2D* body, s16 vx, s16 vy);
// Add gravity and clamp each velocity axis separately for an active dynamic
// body. Null, inactive and static bodies are ignored. Supply a nonnegative limit
// and avoid overflow in the additions before clamping; position is unchanged.
void kq2d_body_apply_gravity(KQBody2D* body, s16 gravity_x, s16 gravity_y, s16 max_speed);
// Scale both velocities through the signed 8-bit coefficient intrinsic.
// Despite the unsigned parameter, values 128..255 become negative coefficients;
// this is not a full-range unsigned Q8 damping helper. Null is ignored.
void kq2d_body_apply_friction(KQBody2D* body, u8 friction_q8);
// Delegate strict rectangle overlap to the fixed-point utility; edge-only
// contact is false. Coordinate-plus-size sums must remain representable as s16.
u8 kq2d_rect_intersect(KQRect a, KQRect b);
// Delegate the half-open point test: include left/top and exclude right/bottom.
// Coordinate-plus-size sums must remain representable as s16.
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
s16 kq2d_scale_q8(s16 value, s16 coefficient);
// Conservative circular bound, with a shared Q8 scale for both components.
// Quantizes toward zero; very small limits relative to input may yield zero.
// Components and positive max_speed must be <= 16383 in magnitude.
void kq2d_body_limit_speed(KQBody2D* body, s16 max_speed);
// Kinematic surface response; returns incoming normal speed, or zero if separating.
// Caller resolves penetration separately. Relative components must fit +/-8191.
s16 kq2d_body_resolve_surface(KQBody2D* body, const KQSurface2D* surface);
// Linear crossing time in Q8 (0..256); positive separation is outside.
// Returns 0 for initial overlap, 256 for no crossing. Gaps must fit +/-8191.
u16 kq2d_surface_toi_q8(s16 start_gap, s16 end_gap);

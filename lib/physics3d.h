#pragma once

// Lightweight deterministic 3D physics for KITAQGB.
// AABB contacts, acceleration, mass-weighted bounce, and impact state flags.

// The game owns break decisions: the solver records impact speeds but never
// sets this flag or deactivates a body based on break_speed. Positions, extents,
// velocities and acceleration use consistent caller-selected per-step units.
#define KQ3D_BODY_BROKEN ((u8)0x01)

typedef struct {
    s16 x;
    s16 y;
    s16 z;
    s16 vx;
    s16 vy;
    s16 vz;
    s16 ax;
    s16 ay;
    s16 az;
    s16 half_x;
    s16 half_y;
    s16 half_z;
    s16 mass_q8;        // 256 means mass 1.0.
    s16 inv_mass_q8;    // 0 means static body.
    s16 restitution_q8; // 256 means perfectly elastic.
    s16 break_speed;    // Reserved for game-side impact thresholds.
    s16 last_impact_speed;
    u8 active;
    u8 flags;
} KQBody3D;

typedef struct {
    KQBody3D* bodies;
    u8 body_count;
    s16 gravity_x;
    s16 gravity_y;
    s16 gravity_z;
    u8 solver_iterations;
    s16 max_speed;
} KQWorld3D;

// Borrow the body array without initializing it. Set gravity (0,24,0), four
// contact passes and a per-axis speed limit of 512. Null world is ignored;
// the caller retains ownership and must keep the array alive.
void kq3d_world_init(KQWorld3D* world, KQBody3D* bodies, u8 body_count);
// Initialize an active centered box with zero velocity/acceleration,
// restitution 224, cleared impact/flags and mass set through the weight helper.
// The caller supplies valid half extents. Null body is ignored.
void kq3d_body_init(KQBody3D* body, s16 x, s16 y, s16 z, s16 half_x, s16 half_y, s16 half_z, s16 mass_q8);
// Store the requested mass; nonpositive mass makes the body static.
// For positive mass compute inverse weight as max(1,32767/max(mass,64)).
// The stored mass retains the original input; this is a relative solver weight,
// not an exact Q8 reciprocal. Null body is ignored.
void kq3d_body_set_mass(KQBody3D* body, s16 mass_q8);
// Integrate active dynamic bodies and reset their impact values, then solve
// each active unordered pair for the configured passes (zero means one).
// Static bodies retain previous impact maxima until the game clears them.
// Acceleration persists; flags and break thresholds remain game-owned.
// This discrete solver shares non-reentrant ASM scratch and can miss fast crossings.
void kq3d_step(KQWorld3D* world);
// Reset impact for one active dynamic body, add gravity and persistent body
// acceleration, clamp velocity per axis and advance position. Static/inactive
// bodies retain impact state. Null inputs are ignored; no contacts are solved.
// Use a nonnegative speed limit and avoid overflow before clamping.
void kq3d_integrate_body(KQWorld3D* world, KQBody3D* body);
// Test strict overlap on all three axes; face-only contact returns false.
// Valid pointers are required and flags are ignored. Center differences and
// extent sums must fit the signed 16-bit magnitude calculations.
u8 kq3d_overlap_aabb(const KQBody3D* a, const KQBody3D* b);

// Sum three signed products, shifting each by eight bits before addition.
// Coefficients are signed bytes scaled by 1/256; the 16-bit sum is not saturated.
s16 kq3d_dot_q8_8(s16 x, s16 y, s16 z, s8 ax, s8 ay, s8 az);

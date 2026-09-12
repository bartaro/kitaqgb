#pragma once

// Lightweight deterministic 3D physics for KITAQGB.
// AABB contacts, acceleration, mass-weighted bounce, and impact state flags.

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

void kq3d_world_init(KQWorld3D* world, KQBody3D* bodies, u8 body_count);
void kq3d_body_init(KQBody3D* body, s16 x, s16 y, s16 z, s16 half_x, s16 half_y, s16 half_z, s16 mass_q8);
void kq3d_body_set_mass(KQBody3D* body, s16 mass_q8);
void kq3d_step(KQWorld3D* world);
void kq3d_integrate_body(KQWorld3D* world, KQBody3D* body);
u8 kq3d_overlap_aabb(const KQBody3D* a, const KQBody3D* b);

// Wrapper around the existing intrinsic to keep gameplay code readable.
s16 kq3d_dot_q8_8(s16 x, s16 y, s16 z, s8 ax, s8 ay, s8 az);

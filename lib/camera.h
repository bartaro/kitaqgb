#pragma once

#include "scroll.h"

// All position, target and bound fields use signed Q8.8 (-128 to just below 128 pixels).
// Pass initialized, non-null storage; world/screen conversion arguments use integer pixels.
typedef struct Camera8_8 {
    s16 fx;
    s16 fy;
    s16 target_fx;
    s16 target_fy;
    s16 min_fx;
    s16 min_fy;
    s16 max_fx;
    s16 max_fy;
    u8 use_target;
    u8 use_bounds;
} Camera8_8;

// Zero all position/target/bound fields and disable targeting and bounds.
void Camera_Init(Camera8_8* cam);
// Replace the current Q8.8 position and immediately apply enabled bounds.
void Camera_Set(Camera8_8* cam, s16 fx, s16 fy);
// Store a Q8.8 target and enable subsequent step-toward-target updates.
void Camera_SetTarget(Camera8_8* cam, s16 fx, s16 fy);
// Disable target following without moving the camera or erasing the stored target.
void Camera_ClearTarget(Camera8_8* cam);
// Install ordered Q8.8 bounds, enable them and clamp the current position immediately.
void Camera_SetBounds(Camera8_8* cam, s16 min_fx, s16 min_fy, s16 max_fx, s16 max_fy);
// Disable clamping while retaining the stored bounds and current position.
void Camera_ClearBounds(Camera8_8* cam);
// Add Q8.8 deltas and clamp afterward. Keep the intermediate sums within s16;
// clamping cannot repair arithmetic that already overflowed.
void Camera_Add(Camera8_8* cam, s16 dfx, s16 dfy);
// Move by the signed target delta shifted by 0..7 bits, then apply bounds.
// Quantization can make a small delta produce no movement; only exact equality
// clears targeting, and a target outside the bounds may remain active indefinitely.
void Camera_StepTowardTarget(Camera8_8* cam, u8 shift);
// Discard fractional bits and write the low eight integer bits to background scroll registers.
void Camera_ApplyBg(const Camera8_8* cam);
// Convert the Q8.8 position to byte scroll values and stage them for a later flush.
void Camera_ApplyBgBuffered(const Camera8_8* cam);
// Derive nonnegative camera extents from world minus screen size, then convert
// them to Q8.8 bounds. Extents must fit signed Q8.8; the u16 input type does
// not imply support for arbitrary large worlds.
void Camera_ClampToSize(Camera8_8* cam, u16 world_w_px, u16 world_h_px, u8 screen_w_px, u8 screen_h_px);
// Convert an integer-pixel world X by subtracting the camera's integer X component.
s16 Camera_WorldToScreenX(const Camera8_8* cam, s16 world_x);
// Convert an integer-pixel world Y by subtracting the camera's integer Y component.
s16 Camera_WorldToScreenY(const Camera8_8* cam, s16 world_y);
// Convert integer-pixel screen X back to world X using the camera's integer component.
s16 Camera_ScreenToWorldX(const Camera8_8* cam, s16 screen_x);
// Convert integer-pixel screen Y back to world Y using the camera's integer component.
s16 Camera_ScreenToWorldY(const Camera8_8* cam, s16 screen_y);

// Set the shared camera from integer pixels, converting to the representable Q8.8 range.
void camera_set(s16 x, s16 y);
// Position the shared camera so the world point appears at the requested screen
// anchor. This is an immediate move, not the smoothed target-following path.
void camera_follow_xy(s16 x, s16 y, u8 screen_cx, u8 screen_cy);
// Derive and apply world/screen bounds to the shared convenience camera.
void camera_clamp(u16 world_w_px, u16 world_h_px, u8 screen_w_px, u8 screen_h_px);
// Write the shared camera's current integer position to background scroll registers.
void camera_apply();
// Convert integer-pixel world X using the shared convenience camera.
s16 camera_world_to_screen_x(s16 world_x);
// Convert integer-pixel world Y using the shared convenience camera.
s16 camera_world_to_screen_y(s16 world_y);

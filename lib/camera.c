#include "camera.h"

// Shared camera used by the lowercase convenience API; the uppercase API
// accepts separate caller-owned cameras. Positions use signed 16-bit Q8.8.
static Camera8_8 camera_default;

// Clamp to an inclusive ordered interval; callers provide lo <= hi.
static s16 Camera_ClampOne(s16 v, s16 lo, s16 hi)
{
    if (v < lo) return lo;
    if (v > hi) return hi;
    return v;
}

// Clamp the current position when bounds are enabled, leaving the target unchanged.
static void Camera_ApplyBounds(Camera8_8* cam)
{
    if (cam->use_bounds == 0) return;
    cam->fx = Camera_ClampOne(cam->fx, cam->min_fx, cam->max_fx);
    cam->fy = Camera_ClampOne(cam->fy, cam->min_fy, cam->max_fy);
}

// Zero all position/target/bound fields and disable targeting and bounds.
void Camera_Init(Camera8_8* cam)
{
    cam->fx = 0;
    cam->fy = 0;
    cam->target_fx = 0;
    cam->target_fy = 0;
    cam->min_fx = 0;
    cam->min_fy = 0;
    cam->max_fx = 0;
    cam->max_fy = 0;
    cam->use_target = 0;
    cam->use_bounds = 0;
}

// Replace the current Q8.8 position and immediately apply enabled bounds.
void Camera_Set(Camera8_8* cam, s16 fx, s16 fy)
{
    cam->fx = fx;
    cam->fy = fy;
    Camera_ApplyBounds(cam);
}

// Store a Q8.8 target and enable subsequent step-toward-target updates.
void Camera_SetTarget(Camera8_8* cam, s16 fx, s16 fy)
{
    cam->target_fx = fx;
    cam->target_fy = fy;
    cam->use_target = 1;
}

// Disable target following without moving the camera or erasing the stored target.
void Camera_ClearTarget(Camera8_8* cam)
{
    cam->use_target = 0;
}

// Install ordered Q8.8 bounds, enable them and clamp the current position immediately.
void Camera_SetBounds(Camera8_8* cam, s16 min_fx, s16 min_fy, s16 max_fx, s16 max_fy)
{
    cam->min_fx = min_fx;
    cam->min_fy = min_fy;
    cam->max_fx = max_fx;
    cam->max_fy = max_fy;
    cam->use_bounds = 1;
    Camera_ApplyBounds(cam);
}

// Disable clamping while retaining the stored bounds and current position.
void Camera_ClearBounds(Camera8_8* cam)
{
    cam->use_bounds = 0;
}

// Add Q8.8 deltas and clamp afterward. Keep the intermediate sums within s16;
// clamping cannot repair arithmetic that already overflowed.
void Camera_Add(Camera8_8* cam, s16 dfx, s16 dfy)
{
    cam->fx += dfx;
    cam->fy += dfy;
    Camera_ApplyBounds(cam);
}

// Move by the signed target delta shifted by 0..7 bits, then apply bounds.
// Quantization can make a small delta produce no movement; only exact equality
// clears targeting, and a target outside the bounds may remain active indefinitely.
void Camera_StepTowardTarget(Camera8_8* cam, u8 shift)
{
    s16 dx;
    s16 dy;

    if (cam->use_target == 0) return;
    if (shift > 7) shift = 7;

    // Keep target-minus-position differences within s16 as well as the individual positions.
    dx = cam->target_fx - cam->fx;
    dy = cam->target_fy - cam->fy;

    if (dx != 0) cam->fx += (dx >> shift);
    if (dy != 0) cam->fy += (dy >> shift);

    if (cam->fx == cam->target_fx && cam->fy == cam->target_fy) cam->use_target = 0;
    Camera_ApplyBounds(cam);
}

// Discard fractional bits and write the low eight integer bits to background scroll registers.
void Camera_ApplyBg(const Camera8_8* cam)
{
    __scroll_bg_set((u8)(cam->fx >> 8), (u8)(cam->fy >> 8));
}

// Convert the Q8.8 position to byte scroll values and stage them for a later flush.
void Camera_ApplyBgBuffered(const Camera8_8* cam)
{
    __scroll_bg_set_buffered((u8)(cam->fx >> 8), (u8)(cam->fy >> 8));
}

// Derive nonnegative camera extents from world minus screen size, then convert
// them to Q8.8 bounds. Extents must fit signed Q8.8; the u16 input type does
// not imply support for arbitrary large worlds.
void Camera_ClampToSize(Camera8_8* cam, u16 world_w_px, u16 world_h_px, u8 screen_w_px, u8 screen_h_px)
{
    s16 max_x = 0;
    s16 max_y = 0;

    if (world_w_px > screen_w_px) max_x = (s16)((u16)(world_w_px - screen_w_px) << 8);
    if (world_h_px > screen_h_px) max_y = (s16)((u16)(world_h_px - screen_h_px) << 8);
    Camera_SetBounds(cam, (s16)0, (s16)0, max_x, max_y);
}

// Convert an integer-pixel world X by subtracting the camera's integer X component.
s16 Camera_WorldToScreenX(const Camera8_8* cam, s16 world_x)
{
    return (s16)(world_x - (cam->fx >> 8));
}

// Convert an integer-pixel world Y by subtracting the camera's integer Y component.
s16 Camera_WorldToScreenY(const Camera8_8* cam, s16 world_y)
{
    return (s16)(world_y - (cam->fy >> 8));
}

// Convert integer-pixel screen X back to world X using the camera's integer component.
s16 Camera_ScreenToWorldX(const Camera8_8* cam, s16 screen_x)
{
    return (s16)(screen_x + (cam->fx >> 8));
}

// Convert integer-pixel screen Y back to world Y using the camera's integer component.
s16 Camera_ScreenToWorldY(const Camera8_8* cam, s16 screen_y)
{
    return (s16)(screen_y + (cam->fy >> 8));
}

// Set the shared camera from integer pixels, converting to the representable Q8.8 range.
void camera_set(s16 x, s16 y)
{
    Camera_Set(&camera_default, (s16)(x << 8), (s16)(y << 8));
}

// Position the shared camera so the world point appears at the requested screen
// anchor. This is an immediate move, not the smoothed target-following path.
void camera_follow_xy(s16 x, s16 y, u8 screen_cx, u8 screen_cy)
{
    s16 cx = (s16)(x - screen_cx);
    s16 cy = (s16)(y - screen_cy);
    Camera_Set(&camera_default, (s16)(cx << 8), (s16)(cy << 8));
}

// Derive and apply world/screen bounds to the shared convenience camera.
void camera_clamp(u16 world_w_px, u16 world_h_px, u8 screen_w_px, u8 screen_h_px)
{
    Camera_ClampToSize(&camera_default, world_w_px, world_h_px, screen_w_px, screen_h_px);
}

// Write the shared camera's current integer position to background scroll registers.
void camera_apply()
{
    Camera_ApplyBg(&camera_default);
}

// Convert integer-pixel world X using the shared convenience camera.
s16 camera_world_to_screen_x(s16 world_x)
{
    return Camera_WorldToScreenX(&camera_default, world_x);
}

// Convert integer-pixel world Y using the shared convenience camera.
s16 camera_world_to_screen_y(s16 world_y)
{
    return Camera_WorldToScreenY(&camera_default, world_y);
}

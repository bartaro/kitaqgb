#pragma once

#include "scroll.h"

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

void Camera_Init(Camera8_8* cam);
void Camera_Set(Camera8_8* cam, s16 fx, s16 fy);
void Camera_SetTarget(Camera8_8* cam, s16 fx, s16 fy);
void Camera_ClearTarget(Camera8_8* cam);
void Camera_SetBounds(Camera8_8* cam, s16 min_fx, s16 min_fy, s16 max_fx, s16 max_fy);
void Camera_ClearBounds(Camera8_8* cam);
void Camera_Add(Camera8_8* cam, s16 dfx, s16 dfy);
void Camera_StepTowardTarget(Camera8_8* cam, u8 shift);
void Camera_ApplyBg(const Camera8_8* cam);
void Camera_ApplyBgBuffered(const Camera8_8* cam);
void Camera_ClampToSize(Camera8_8* cam, u16 world_w_px, u16 world_h_px, u8 screen_w_px, u8 screen_h_px);
s16 Camera_WorldToScreenX(const Camera8_8* cam, s16 world_x);
s16 Camera_WorldToScreenY(const Camera8_8* cam, s16 world_y);
s16 Camera_ScreenToWorldX(const Camera8_8* cam, s16 screen_x);
s16 Camera_ScreenToWorldY(const Camera8_8* cam, s16 screen_y);

void camera_set(s16 x, s16 y);
void camera_follow_xy(s16 x, s16 y, u8 screen_cx, u8 screen_cy);
void camera_clamp(u16 world_w_px, u16 world_h_px, u8 screen_w_px, u8 screen_h_px);
void camera_apply();
s16 camera_world_to_screen_x(s16 world_x);
s16 camera_world_to_screen_y(s16 world_y);

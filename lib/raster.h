#pragma once

#include "scroll.h"

#define KQ_RASTER_MAX_BANDS 8

#define KQ_RASTER_OK        0
#define KQ_RASTER_FULL      1
#define KQ_RASTER_BAD_LY    2
#define KQ_RASTER_UNSORTED  3
#define KQ_RASTER_BAD_RANGE 4

#define KQ_RASTER_BG        KQ_SCROLL_SPLIT_USE_BG
#define KQ_RASTER_WIN       KQ_SCROLL_SPLIT_USE_WIN
#define KQ_RASTER_WIN_SHOW  KQ_SCROLL_SPLIT_WIN_SHOW
#define KQ_RASTER_WIN_HIDE  KQ_SCROLL_SPLIT_WIN_HIDE
#define KQ_RASTER_BG_COLOR0 KQ_SCROLL_SPLIT_BG_COLOR0
#define KQ_RASTER_FLAG_MASK 0x1F

#define KQ_RASTER_LINE_FLAT        0
#define KQ_RASTER_LINE_X_TITLE     1
#define KQ_RASTER_LINE_TRAVEL_GATE 2

typedef struct RasterLineX {
    u8 profile;
    u8 base_x;
    u8 phase;
    u8 phase_step;
    u8 frame_divider;
    u8 frame_counter;
} RasterLineX;

void Raster_Init();
void Raster_Clear();
void Raster_Disable();

u8 Raster_GetCount();
u8 Raster_GetLastError();

u8 Raster_Push(u8 ly, u8 scx, u8 scy);
u8 Raster_PushEx(u8 ly, u8 scx, u8 scy, u8 wx, u8 wy, u8 flags);

u8 Raster_PushWindowRaw(u8 ly, u8 wx, u8 wy, u8 flags);
u8 Raster_PushWindowScreen(u8 ly, u8 screen_x, u8 screen_y, u8 flags);
u8 Raster_PushBgWindowScreen(u8 ly, u8 scx, u8 scy, u8 screen_x, u8 screen_y, u8 flags);
u8 Raster_PushBgColor0(u8 ly, u16 rgb15);

u8 Raster_Commit();

u8 Raster_BuildHudTop(u8 hud_height, u8 camera_x, u8 camera_y);
u8 Raster_BuildHudBottom(u8 hud_y, u8 camera_x, u8 camera_y);

u8 Raster_BuildParallax2(u8 split_y, u8 far_scx, u8 near_scx, u8 scy);
u8 Raster_BuildParallax3(u8 y1, u8 y2, u8 scx0, u8 scx1, u8 scx2, u8 scy);

void Raster_LineXInit(RasterLineX* effect, u8 profile, u8 base_x);
void Raster_LineXSetProfile(RasterLineX* effect, u8 profile);
void Raster_LineXSetSpeed(RasterLineX* effect, u8 phase_step, u8 frame_divider);
void Raster_LineXSetPhase(RasterLineX* effect, u8 phase);
u8 Raster_LineXGetOffset(const RasterLineX* effect, u8 ly);
void Raster_LineXRunFrame(RasterLineX* effect);

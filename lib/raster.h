#pragma once

#include "scroll.h"

// Keep wrapper capacity consistent with the compiler split-table capacity; wrapper checks do not enlarge it.
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

// Caller-owned wave state. Initialize it before use and keep it non-null for every LineX operation.
// frame_counter measures RunFrame calls, not an independent hardware clock.
typedef struct RasterLineX {
    u8 profile;
    u8 base_x;
    u8 phase;
    u8 phase_step;
    u8 frame_divider;
    u8 frame_counter;
} RasterLineX;

// Stop split scheduling, empty the shared table and clear error/count state.
void Raster_Init();
// Stop splits immediately and clear bookkeeping. Build and commit a new table to restart.
void Raster_Clear();
// Commit an empty split table so no raster bands remain active.
void Raster_Disable();

// Read the number of bands currently recorded by this wrapper.
u8 Raster_GetCount();
// Read the last stored status without clearing it.
u8 Raster_GetLastError();

// Append a validated background-scroll band and advance the monotonic-LY
// bookkeeping. A successful append clears the previous error.
u8 Raster_Push(u8 ly, u8 scx, u8 scy);
// Append a validated extended band after masking flags to supported operations.
u8 Raster_PushEx(u8 ly, u8 scx, u8 scy, u8 wx, u8 wy, u8 flags);

// Enable window updates and pass hardware WX/WY values without coordinate conversion.
u8 Raster_PushWindowRaw(u8 ly, u8 wx, u8 wy, u8 flags);
// Convert screen X to hardware WX by adding seven, then queue a window band.
// The byte addition wraps if callers supply an excessive X coordinate.
u8 Raster_PushWindowScreen(u8 ly, u8 screen_x, u8 screen_y, u8 flags);
// Queue both background and window updates, converting window screen X to WX.
u8 Raster_PushBgWindowScreen(u8 ly, u8 scx, u8 scy, u8 screen_x, u8 screen_y, u8 flags);
// Pack a 15-bit color into the extended band's two scroll-byte fields and select
// the color-zero operation, which interprets those fields as color data.
u8 Raster_PushBgColor0(u8 ly, u16 rgb15);

// Publish pending splits and return the stored status. This commits even if an
// earlier append failed; callers should inspect errors while building the table.
u8 Raster_Commit();

// Validate the HUD height, then replace pending splits with a fixed top band
// and scrolling playfield. The new table still requires an explicit commit.
u8 Raster_BuildHudTop(u8 hud_height, u8 camera_x, u8 camera_y);
// Build a scrolling upper playfield and fixed bottom HUD after validating the split.
u8 Raster_BuildHudBottom(u8 hud_y, u8 camera_x, u8 camera_y);

// Build two horizontal-scroll bands sharing one vertical scroll value.
u8 Raster_BuildParallax2(u8 split_y, u8 far_scx, u8 near_scx, u8 scy);
// Build three horizontal-scroll bands at ordered visible split lines. Invalid
// parameters leave the previous pending table intact and set the error status.
u8 Raster_BuildParallax3(u8 y1, u8 y2, u8 scx0, u8 scx1, u8 scx2, u8 scy);

// Initialize a scanline-wave state with phase zero and one phase step per frame.
void Raster_LineXInit(RasterLineX* effect, u8 profile, u8 base_x);
// Change the wave profile and restart its phase/divider counter.
void Raster_LineXSetProfile(RasterLineX* effect, u8 profile);
// Configure the phase increment and frame interval; clamp a zero interval to one.
void Raster_LineXSetSpeed(RasterLineX* effect, u8 phase_step, u8 frame_divider);
// Set the byte phase directly without resetting the frame-divider counter.
void Raster_LineXSetPhase(RasterLineX* effect, u8 phase);
// Look up the selected periodic profile at LY + phase and add base X modulo
// 256. An unknown profile returns the unmodified base X.
u8 Raster_LineXGetOffset(const RasterLineX* effect, u8 ly);
// Render all 144 visible lines with one SCX write per line, then advance phase.
// This foreground renderer masks interrupts during the visible frame. Do game
// and audio work between calls; do not combine it with a STAT raster driver.
// Returns without changing the effect when the LCD is off.
void Raster_LineXRunFrame(RasterLineX* effect);

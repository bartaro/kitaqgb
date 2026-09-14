#pragma once

// Flags select operations in a raster split; raw window X includes the hardware +7 offset.
#define KQ_SCROLL_SPLIT_USE_BG   0x01
#define KQ_SCROLL_SPLIT_USE_WIN  0x02
#define KQ_SCROLL_SPLIT_WIN_SHOW 0x04
#define KQ_SCROLL_SPLIT_WIN_HIDE 0x08
#define KQ_SCROLL_SPLIT_BG_COLOR0 0x10

void __scroll_bg_set(u8 scx, u8 scy);
void __scroll_bg_x_set(u8 scx);
void __scroll_bg_y_set(u8 scy);
u8 __scroll_bg_x_get();
u8 __scroll_bg_y_get();

void __scroll_win_set(u8 wx, u8 wy);
void __scroll_win_x_set(u8 wx);
void __scroll_win_y_set(u8 wy);
u8 __scroll_win_x_get();
u8 __scroll_win_y_get();

void __scroll_bg_set_buffered(u8 scx, u8 scy);
void __scroll_bg_x_set_buffered(u8 scx);
void __scroll_bg_y_set_buffered(u8 scy);
void __scroll_win_set_buffered(u8 wx, u8 wy);
void __scroll_win_x_set_buffered(u8 wx);
void __scroll_win_y_set_buffered(u8 wy);
void __scroll_flush();

void __scroll_split_reset();
void __scroll_split_push(u8 ly, u8 scx, u8 scy);
void __scroll_split_push_ex(u8 ly, u8 scx, u8 scy, u8 wx, u8 wy, u8 flags);
void __scroll_split_commit();

void __scroll_win_show();
void __scroll_win_hide();
void __scroll_bg_add(s8 dx, s8 dy);
void __scroll_win_add(s8 dx, s8 dy);

// Reset background scroll to zero and place the window at screen origin
// using hardware WX=7 and WY=0.
void Scroll_Init();
// Write both hardware background-scroll bytes immediately.
void Scroll_SetBg(u8 scx, u8 scy);
// Write horizontal background scroll while leaving vertical scroll unchanged.
void Scroll_SetBgX(u8 scx);
// Write vertical background scroll while leaving horizontal scroll unchanged.
void Scroll_SetBgY(u8 scy);
// Read the current horizontal background-scroll byte through the intrinsic.
u8 Scroll_GetBgX();
// Read the current vertical background-scroll byte through the intrinsic.
u8 Scroll_GetBgY();

// Write raw hardware WX/WY values; callers add seven to a screen-space window X.
void Scroll_SetWindow(u8 wx, u8 wy);
// Write raw hardware WX without converting a screen-space X coordinate.
void Scroll_SetWindowX(u8 wx);
// Write the window Y coordinate through the intrinsic.
void Scroll_SetWindowY(u8 wy);
// Read raw hardware WX, including its seven-pixel coordinate offset.
u8 Scroll_GetWindowX();
// Read the current hardware window Y value.
u8 Scroll_GetWindowY();

// Stage background-scroll values in the intrinsic buffer for a later flush.
void Scroll_SetBgBuffered(u8 scx, u8 scy);
// Stage raw window-coordinate values for a later flush.
void Scroll_SetWindowBuffered(u8 wx, u8 wy);
// Apply the buffered scroll/window state through the compiler runtime.
void Scroll_Flush();

// Apply signed deltas to the byte-sized background-scroll coordinates.
void Scroll_BgAdd(s8 dx, s8 dy);
// Apply signed deltas to the byte-sized hardware window coordinates.
void Scroll_WindowAdd(s8 dx, s8 dy);
// Request window visibility through the compiler intrinsic.
void Scroll_WindowShow();
// Request that the window be hidden through the compiler intrinsic.
void Scroll_WindowHide();

// Clear the pending split table before building a new raster configuration.
void Scroll_SplitReset();
// Append a background-scroll split directly. Use Raster_Push when caller-side
// scanline ordering and capacity validation are desired.
void Scroll_SplitPush(u8 ly, u8 scx, u8 scy);
// Append an extended split with raw background/window fields and operation flags.
void Scroll_SplitPushEx(u8 ly, u8 scx, u8 scy, u8 wx, u8 wy, u8 flags);
// Reuse the two background-scroll fields to carry the low/high RGB15 bytes
// for a split whose operation flag selects background color zero.
void Scroll_SplitPushBgColor0(u8 ly, u16 rgb15);
// Publish the pending split configuration through the compiler runtime.
void Scroll_SplitCommit();

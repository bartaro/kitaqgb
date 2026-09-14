#include "scroll.h"

// Reset background scroll to zero and place the window at screen origin
// using hardware WX=7 and WY=0.
void Scroll_Init()
{
    __scroll_bg_set(0, 0);
    __scroll_win_set(7, 0);
}

// Write both hardware background-scroll bytes immediately.
void Scroll_SetBg(u8 scx, u8 scy) { __scroll_bg_set(scx, scy); }
// Write horizontal background scroll while leaving vertical scroll unchanged.
void Scroll_SetBgX(u8 scx) { __scroll_bg_x_set(scx); }
// Write vertical background scroll while leaving horizontal scroll unchanged.
void Scroll_SetBgY(u8 scy) { __scroll_bg_y_set(scy); }
// Read the current horizontal background-scroll byte through the intrinsic.
u8 Scroll_GetBgX() { return __scroll_bg_x_get(); }
// Read the current vertical background-scroll byte through the intrinsic.
u8 Scroll_GetBgY() { return __scroll_bg_y_get(); }

// Write raw hardware WX/WY values; callers add seven to a screen-space window X.
void Scroll_SetWindow(u8 wx, u8 wy) { __scroll_win_set(wx, wy); }
// Write raw hardware WX without converting a screen-space X coordinate.
void Scroll_SetWindowX(u8 wx) { __scroll_win_x_set(wx); }
// Write the window Y coordinate through the intrinsic.
void Scroll_SetWindowY(u8 wy) { __scroll_win_y_set(wy); }
// Read raw hardware WX, including its seven-pixel coordinate offset.
u8 Scroll_GetWindowX() { return __scroll_win_x_get(); }
// Read the current hardware window Y value.
u8 Scroll_GetWindowY() { return __scroll_win_y_get(); }

// Stage background-scroll values in the intrinsic buffer for a later flush.
void Scroll_SetBgBuffered(u8 scx, u8 scy) { __scroll_bg_set_buffered(scx, scy); }
// Stage raw window-coordinate values for a later flush.
void Scroll_SetWindowBuffered(u8 wx, u8 wy) { __scroll_win_set_buffered(wx, wy); }
// Apply the buffered scroll/window state through the compiler runtime.
void Scroll_Flush() { __scroll_flush(); }

// Apply signed deltas to the byte-sized background-scroll coordinates.
void Scroll_BgAdd(s8 dx, s8 dy) { __scroll_bg_add(dx, dy); }
// Apply signed deltas to the byte-sized hardware window coordinates.
void Scroll_WindowAdd(s8 dx, s8 dy) { __scroll_win_add(dx, dy); }
// Request window visibility through the compiler intrinsic.
void Scroll_WindowShow() { __scroll_win_show(); }
// Request that the window be hidden through the compiler intrinsic.
void Scroll_WindowHide() { __scroll_win_hide(); }

// Clear the pending split table before building a new raster configuration.
void Scroll_SplitReset() { __scroll_split_reset(); }
// Append a background-scroll split directly. Use Raster_Push when caller-side
// scanline ordering and capacity validation are desired.
void Scroll_SplitPush(u8 ly, u8 scx, u8 scy) { __scroll_split_push(ly, scx, scy); }
// Append an extended split with raw background/window fields and operation flags.
void Scroll_SplitPushEx(u8 ly, u8 scx, u8 scy, u8 wx, u8 wy, u8 flags)
{
    __scroll_split_push_ex(ly, scx, scy, wx, wy, flags);
}
// Reuse the two background-scroll fields to carry the low/high RGB15 bytes
// for a split whose operation flag selects background color zero.
void Scroll_SplitPushBgColor0(u8 ly, u16 rgb15)
{
    __scroll_split_push_ex(ly, (u8)rgb15, (u8)(rgb15 >> 8), 0, 0, KQ_SCROLL_SPLIT_BG_COLOR0);
}
// Publish the pending split configuration through the compiler runtime.
void Scroll_SplitCommit() { __scroll_split_commit(); }

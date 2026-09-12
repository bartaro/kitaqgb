#include "scroll.h"

void Scroll_Init()
{
    __scroll_bg_set(0, 0);
    __scroll_win_set(7, 0);
}

void Scroll_SetBg(u8 scx, u8 scy) { __scroll_bg_set(scx, scy); }
void Scroll_SetBgX(u8 scx) { __scroll_bg_x_set(scx); }
void Scroll_SetBgY(u8 scy) { __scroll_bg_y_set(scy); }
u8 Scroll_GetBgX() { return __scroll_bg_x_get(); }
u8 Scroll_GetBgY() { return __scroll_bg_y_get(); }

void Scroll_SetWindow(u8 wx, u8 wy) { __scroll_win_set(wx, wy); }
void Scroll_SetWindowX(u8 wx) { __scroll_win_x_set(wx); }
void Scroll_SetWindowY(u8 wy) { __scroll_win_y_set(wy); }
u8 Scroll_GetWindowX() { return __scroll_win_x_get(); }
u8 Scroll_GetWindowY() { return __scroll_win_y_get(); }

void Scroll_SetBgBuffered(u8 scx, u8 scy) { __scroll_bg_set_buffered(scx, scy); }
void Scroll_SetWindowBuffered(u8 wx, u8 wy) { __scroll_win_set_buffered(wx, wy); }
void Scroll_Flush() { __scroll_flush(); }

void Scroll_BgAdd(s8 dx, s8 dy) { __scroll_bg_add(dx, dy); }
void Scroll_WindowAdd(s8 dx, s8 dy) { __scroll_win_add(dx, dy); }
void Scroll_WindowShow() { __scroll_win_show(); }
void Scroll_WindowHide() { __scroll_win_hide(); }

void Scroll_SplitReset() { __scroll_split_reset(); }
void Scroll_SplitPush(u8 ly, u8 scx, u8 scy) { __scroll_split_push(ly, scx, scy); }
void Scroll_SplitPushEx(u8 ly, u8 scx, u8 scy, u8 wx, u8 wy, u8 flags)
{
    __scroll_split_push_ex(ly, scx, scy, wx, wy, flags);
}
void Scroll_SplitPushBgColor0(u8 ly, u16 rgb15)
{
    __scroll_split_push_ex(ly, (u8)rgb15, (u8)(rgb15 >> 8), 0, 0, KQ_SCROLL_SPLIT_BG_COLOR0);
}
void Scroll_SplitCommit() { __scroll_split_commit(); }

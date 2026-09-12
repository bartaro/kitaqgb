#pragma once

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

void Scroll_Init();
void Scroll_SetBg(u8 scx, u8 scy);
void Scroll_SetBgX(u8 scx);
void Scroll_SetBgY(u8 scy);
u8 Scroll_GetBgX();
u8 Scroll_GetBgY();

void Scroll_SetWindow(u8 wx, u8 wy);
void Scroll_SetWindowX(u8 wx);
void Scroll_SetWindowY(u8 wy);
u8 Scroll_GetWindowX();
u8 Scroll_GetWindowY();

void Scroll_SetBgBuffered(u8 scx, u8 scy);
void Scroll_SetWindowBuffered(u8 wx, u8 wy);
void Scroll_Flush();

void Scroll_BgAdd(s8 dx, s8 dy);
void Scroll_WindowAdd(s8 dx, s8 dy);
void Scroll_WindowShow();
void Scroll_WindowHide();

void Scroll_SplitReset();
void Scroll_SplitPush(u8 ly, u8 scx, u8 scy);
void Scroll_SplitPushEx(u8 ly, u8 scx, u8 scy, u8 wx, u8 wy, u8 flags);
void Scroll_SplitPushBgColor0(u8 ly, u16 rgb15);
void Scroll_SplitCommit();

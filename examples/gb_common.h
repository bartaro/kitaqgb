#pragma once
// Include this small support file once per program.
#include "font_gb.h"
__location(0xFF40) u8 M_LCDC;
__location(0xFF47) u8 M_BGP;
__location(0xFF48) u8 M_OBP0;
void __wait_vblank();
void __vram_fill(u16 dst, u8 value, u16 len);
void __vram_copy(u16 dst, const void* src, u16 len);
void __settile_xy(u8 x, u8 y, u8 tile);
void m_init() {
    __wait_vblank(); M_LCDC = 0; M_BGP = 0xE4; M_OBP0 = 0xE4;
    __vram_copy(0x8000, manual_font, 2048);
    __vram_fill(0x9800, 0, 1024); M_LCDC = 0x91;
}
void m_put(u8 x,u8 y,u8 ch) { __settile_xy(x,y,ch); }
void m_wait() { __wait_vblank(); }
void m_text(u8 x,u8 y,const u8* text) {
    while (*text != 0) { m_put(x,y,*text); x++; text++; }
}
void m_number(u8 value) {
    m_put(3,8,(u8)('0'+value/100));
    m_put(4,8,(u8)('0'+(value/10)%10));
    m_put(5,8,(u8)('0'+value%10));
}

// 背景スクロール
// Expected: title moves left as SCX increases
#include "gb_common.h"
#include "scroll.h"
u8 phase;
void main() {
    m_init();
    m_text(2,3,"SCROLL");
    Scroll_SetBg(0,0);
    while (1) { m_wait(); phase++; Scroll_SetBg(phase,0); }
}

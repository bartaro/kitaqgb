// Scroll the background horizontally.
// Expected: title moves left as SCX increases
#include "gb_common.h"
#include "scroll.h"
u8 phase;
// Advance an 8-bit horizontal scroll position once per frame, wrapping naturally
// after 255. Background motion demonstrates the changing scroll register.
void main() {
    m_init();
    m_text(2,3,"SCROLL");
    Scroll_SetBg(0,0);
    while (1) { m_wait(); phase++; Scroll_SetBg(phase,0); }
}

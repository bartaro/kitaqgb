// CGBの色指定
// Expected: 042 in purple on white; CGB mode
#include "gb_common.h"
#include "cgb_palette.h"

void main() {
    m_init();
    m_text(2,3,"CGB PALETTE");
    cgb_bg_rgb(0,0,31,31,31); cgb_bg_rgb(0,3,12,0,22);
    m_number(42);
    while (1) { m_wait();  }
}

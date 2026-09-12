// VRAM更新キュー
// Expected: 4 at (3,8)
#include "gb_common.h"
#include "vram.h"

void main() {
    m_init();
    m_text(2,3,"VRAM QUEUE");
    vram_init();
    vram_queue_bg_tile(3,8,52); vram_flush();
    while (1) { m_wait();  }
}

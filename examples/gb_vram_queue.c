// Queue a VRAM update and flush it at VBlank.
// Expected: 4 at (3,8)
#include "gb_common.h"
#include "vram.h"

// Queue font tile 52 (the digit 4) at tile coordinate (3,8), then wait for
// VBlank and execute the queued transfer.
void main() {
    m_init();
    m_text(2,3,"VRAM QUEUE");
    vram_init();
    vram_queue_bg_tile(3,8,52); vram_flush();
    while (1) { m_wait();  }
}

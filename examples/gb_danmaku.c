// Allocate a bullet and inspect the pool count.
// Expected: 001; numeric pool example, no bullet BG compositor
#include "gb_common.h"
#include "danmaku.h"

// Reset the bullet pool, spawn one bullet and display the active count.
// This numeric example does not install the background bullet compositor.
void main() {
    m_init();
    m_text(2,3,"DANMAKU");
    danmaku_reset(); danmaku_spawn(40,40,16,0);
    m_number(dm_count);
    while (1) { m_wait();  }
}

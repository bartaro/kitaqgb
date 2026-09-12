// フレームカウンター
// Expected: 002
#include "gb_common.h"
#include "system.h"

void main() {
    m_init();
    m_text(2,3,"SYSTEM");
    system_init(); system_wait_vblank(); system_wait_vblank();
    m_number(system_get_frame8());
    while (1) { m_wait();  }
}

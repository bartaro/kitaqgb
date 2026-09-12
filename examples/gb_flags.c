// フラグとクエスト状態
// Expected: 001
#include "gb_common.h"
#include "rpg.h"

void main() {
    m_init();
    m_text(2,3,"FLAGS");
    flag_clear(12); flag_set(12); m_number(flag_get(12));
    while (1) { m_wait();  }
}

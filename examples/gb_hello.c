// 最初の画面表示
// Expected: HELLO WORLD and 042
#include "gb_common.h"


void main() {
    m_init();
    m_text(2,3,"HELLO");
    m_text(2,5,"HELLO WORLD");
    m_number(42);
    while (1) { m_wait();  }
}

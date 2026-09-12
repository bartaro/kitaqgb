// 変数・算術・型変換
// Expected: 030
#include "gb_common.h"


void main() {
    m_init();
    m_text(2,3,"ARITHMETIC");
    u16 sum;
    sum = (u16)200 + 100;
    m_number((u8)(sum / 10));
    while (1) { m_wait();  }
}

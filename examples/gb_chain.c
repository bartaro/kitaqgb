// 座標履歴のリングバッファ
// Expected: 042; history contains (20,30)
#include "gb_common.h"
#include "chain.h"
Chain trail; ChainPoint points[4];
void main() {
    m_init();
    m_text(2,3,"CHAIN");
    chain_init(&trail,points,4); chain_push_head(&trail,20,30);
    m_number(42);
    while (1) { m_wait();  }
}

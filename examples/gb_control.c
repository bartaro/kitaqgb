// 条件分岐と繰り返し
// Expected: 042
#include "gb_common.h"


void main() {
    m_init();
    m_text(2,3,"CONTROL");
    u8 i; u8 sum; sum=0;
    for (i=0;i<8;i++) { if (i==3) continue; sum=(u8)(sum+i); }
    while (sum<30) { sum++; }
    do { sum++; } while (sum<32);
    switch (sum) { case 32: sum=42; break; default: sum=0; break; }
    m_number(sum);
    while (1) { m_wait();  }
}

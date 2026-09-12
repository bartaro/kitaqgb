// 固定小数点Q8.8
// Expected: 012
#include "gb_common.h"
#include "fixed.h"

void main() {
    m_init();
    m_text(2,3,"FIXED");
    fix8 a; fix8 b; a=fix_from_int(3); b=fix_from_int(4);
    m_number((u8)fix_to_int(fix_mul(a,b)));
    while (1) { m_wait();  }
}

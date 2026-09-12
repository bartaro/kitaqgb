// 値の記録とアサート
// Expected: 001; trace log contains HP=42
#include "gb_common.h"
#include "debug.h"

void main() {
    m_init();
    m_text(2,3,"DEBUG");
    debug_init(); debug_set_frame(1); debug_trace_u8("HP",42);
    m_number(debug_get_trace_count());
    while (1) { m_wait();  }
}

// Record diagnostic values in the debug trace.
// Expected: 001; trace log contains HP=42
#include "gb_common.h"
#include "debug.h"

// Record HP = 42 with frame tag 1 and display the number of trace entries.
// The trace buffer contains the value; the screen displays the count, not HP.
void main() {
    m_init();
    m_text(2,3,"DEBUG");
    debug_init(); debug_set_frame(1); debug_trace_u8("HP",42);
    m_number(debug_get_trace_count());
    while (1) { m_wait();  }
}

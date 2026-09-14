// Variables, arithmetic and type conversion.
// Expected: 030
#include "gb_common.h"


// Widen before adding 200 + 100, divide the 16-bit sum, and display 30.
// The cast demonstrates avoiding an 8-bit intermediate result.
void main() {
    m_init();
    m_text(2,3,"ARITHMETIC");
    u16 sum;
    sum = (u16)200 + 100;
    m_number((u8)(sum / 10));
    while (1) { m_wait();  }
}

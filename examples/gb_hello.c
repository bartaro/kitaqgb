// Display a first message on screen.
// Expected: HELLO WORLD and 042
#include "gb_common.h"


// Initialize the display/font, write HELLO WORLD and a three-digit 042,
// then wait for frames so the result remains visible.
void main() {
    m_init();
    m_text(2,3,"HELLO");
    m_text(2,5,"HELLO WORLD");
    m_number(42);
    while (1) { m_wait();  }
}

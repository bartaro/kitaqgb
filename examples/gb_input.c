// Distinguish a new button press from a held button.
// Expected: 000 initially; A increments once per press
#include "gb_common.h"
#include "input.h"
u8 manual_count;
// Sample buttons once per frame and increment only on a new A press.
// Holding A does not repeatedly increment; the byte-sized counter wraps after 255.
void main() {
    m_init();
    m_text(2,3,"INPUT");
    input_init();
    m_number(0);
    while (1) { m_wait(); input_update(); if(input_pressed(BTN_A)) { manual_count++; m_number(manual_count); } }
}

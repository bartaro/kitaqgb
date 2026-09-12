// ボタン入力と押した瞬間
// Expected: 000 initially; A increments once per press
#include "gb_common.h"
#include "input.h"
u8 manual_count;
void main() {
    m_init();
    m_text(2,3,"INPUT");
    input_init();
    m_number(0);
    while (1) { m_wait(); input_update(); if(input_pressed(BTN_A)) { manual_count++; m_number(manual_count); } }
}

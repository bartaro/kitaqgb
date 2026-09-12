// 盤面の読み書き
// Expected: 042
#include "gb_common.h"
#include "slg.h"
SLGBoard board; u8 cells[16];
void main() {
    m_init();
    m_text(2,3,"BOARD");
    slg_board_init(&board,4,4,cells); slg_board_clear(&board,0);
    slg_board_set(&board,2,1,42); m_number(slg_board_get(&board,2,1));
    while (1) { m_wait();  }
}

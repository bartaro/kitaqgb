// バンク番号付きデータ
// Expected: 042; bank 0 fixed-ROM data copy
#include "gb_common.h"
#include "bank.h"
__prg_rom u8 values[] = {7,42}; u8 copied[2];
void main() {
    m_init();
    m_text(2,3,"BANK");
    far_data_read(0,values,copied,2); m_number(copied[1]);
    while (1) { m_wait();  }
}

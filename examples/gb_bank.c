// Read data using an explicit ROM bank.
// Expected: 042; explicit bank argument 0
#include "gb_common.h"
#include "bank.h"
__prg_rom u8 values[] = {7,42}; u8 copied[2];
// Copy two ROM bytes using bank argument zero and display the second byte.
// This introduces the API; it does not test switching between distinct payload banks.
void main() {
    m_init();
    m_text(2,3,"BANK");
    far_data_read(0,values,copied,2); m_number(copied[1]);
    while (1) { m_wait();  }
}

// Set Game Boy Color palette entries.
// Expected: 042 in purple on white; CGB mode
#include "gb_common.h"
#include "cgb_palette.h"

// Set palette-zero background colors to white and purple, then display 42.
// Run the ROM in CGB mode to observe the color result.
void main() {
    m_init();
    m_text(2,3,"CGB PALETTE");
    // Disable the LCD during VBlank so both palette bytes remain accessible.
    m_wait(); M_LCDC = 0;
    cgb_bg_rgb(0,0,31,31,31); cgb_bg_rgb(0,3,12,0,22);
    M_LCDC = 0x91;
    m_number(42);
    while (1) { m_wait();  }
}

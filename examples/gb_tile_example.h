#pragma once
// Shared startup and result display for the tile-operation examples.
// The supplied 92-glyph ASCII font is loaded by gb_common.h.
#include "gb_common.h"
#include "cgb_tile.h"

__location(0xFF68) u8 T_BGPI;
__location(0xFF69) u8 T_BGPD;

// Initialize both maps and CGB attribute banks while the LCD is stopped.
// The maps selected for BG and window are deliberately different from $9800.
void tile_example_begin() {
    u8 i;
    m_init();
    m_wait(); M_LCDC = 0;
    __cgb_safe_set_vbk(0);
    __vram_fill(0x9800, 0, 2048);
    if (__cgb_is_cgb()) {
        __cgb_safe_set_vbk(1);
        __vram_fill(0x9800, 0, 2048);
        __cgb_safe_set_vbk(0);
        T_BGPI = 0x80;
        // Palette 0 uses black; palettes 1, 2 and 3 use red, blue and green.
        // This makes CGB attribute changes visible in the example's characters.
        for (i = 0; i < 8; i++) {
            T_BGPD = 0xFF; T_BGPD = 0x7F;
            T_BGPD = 0xB5; T_BGPD = 0x56;
            T_BGPD = 0x4A; T_BGPD = 0x29;
            if (i == 1) { T_BGPD = 31; T_BGPD = 0; }
            else if (i == 2) { T_BGPD = 0; T_BGPD = 0x7C; }
            else if (i == 3) { T_BGPD = 0; T_BGPD = 2; }
            else { T_BGPD = 0; T_BGPD = 0; }
        }
    }
    M_LCDC = 0x48;
}

// Read a map byte while the LCD is off, then restore the tile-number bank.
u8 tile_example_read(u16 address, u8 bank) {
    u8 value;
    __cgb_safe_set_vbk(bank);
    value = *((u8*)address);
    __cgb_safe_set_vbk(0);
    return value;
}

// Show the selected map and a reproducible result. 000 means no failed checks.
// The example's write remains visible below the title and above the result.
void tile_example_finish(u8 failures, u8 use_map_9c00) {
    __cgb_safe_set_vbk(0);
    if (use_map_9c00) { M_LCDC = 0x99; }
    else { M_LCDC = 0x91; }
    m_text(2,0,"TILE EXAMPLE");
    m_text(2,7,"FAILED CHECKS");
    m_number(failures);
    while (1) { m_wait(); }
}

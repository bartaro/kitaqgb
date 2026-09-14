// Display a sprite through the OAM DMA path.
// Expected: letter A at sprite position
#include "gb_common.h"
#include "sprite.h"

// Allocate the first sprite, choose font tile 65 (A), set its position and
// enable sprite display. Flush shadow OAM after each frame wait.
void main() {
    m_init();
    m_text(2,3,"SPRITE");
    // The freshly reset pool guarantees slot zero for this first allocation.
    // In a running game, check the allocation-failure sentinel before using the returned ID.
    sprite_init(); sprite_set_tile(sprite_alloc(),65);
    sprite_set_pos(0,70,80); M_LCDC=(u8)(M_LCDC|2);
    while (1) { m_wait(); sprite_flush_oam_now(); }
}

// スプライトとOAM DMA
// Expected: letter A at sprite position
#include "gb_common.h"
#include "sprite.h"

void main() {
    m_init();
    m_text(2,3,"SPRITE");
    sprite_init(); sprite_set_tile(sprite_alloc(),65);
    sprite_set_pos(0,70,80); M_LCDC=(u8)(M_LCDC|2);
    while (1) { m_wait(); sprite_flush_oam_now(); }
}

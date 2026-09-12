// 連長圧縮データの展開
// Expected: 042; three bytes each equal to 42
#include "gb_common.h"
#include "rpg.h"
__prg_rom u8 packed[] = {3,42,0};
void main() {
    m_init();
    m_text(2,3,"RLE");
    u8 out[3]; rle_decode(out,packed); m_number(out[2]);
    while (1) { m_wait();  }
}

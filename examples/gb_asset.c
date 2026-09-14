// Load asset bytes into RAM by asset ID.
// Expected: 042
#include "gb_common.h"
#include "asset.h"
AssetDesc table[1]; __prg_rom u8 values[] = {7,42}; u8 copied[2];
// Register a two-byte raw asset, copy it into RAM and display its second byte.
// This example uses bank zero and a destination sized for the complete payload.
void main() {
    m_init();
    m_text(2,3,"ASSET");
    table[0].type=ASSET_TYPE_RAW; table[0].bank=0;
    table[0].ptr=values; table[0].len=2;
    asset_set_table(0,table,1); asset_load_raw(0,copied,2);
    m_number(copied[1]);
    while (1) { m_wait();  }
}

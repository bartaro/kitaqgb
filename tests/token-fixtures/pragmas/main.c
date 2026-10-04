#pragma bank 12
#pragma fixed_bank -1
#pragma fixed_order +2
#pragma hram
#pragma wram0
#pragma wramx
#pragma wramx_bank 0x3
#pragma align 16
#pragma section "physics"
#pragma cgb_palette TEST #FFFFFF #FF0000 0x3e0 $7c00
#pragma rom_title "RUST TEST"
#pragma cgb cgb_only
#pragma cart mbc5
#pragma romsize 128k
#pragma header_logo off
int value;

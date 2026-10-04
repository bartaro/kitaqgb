#pragma bank 2
__prg_rom u8 encoded[5]={3,7,2,9,0};
#pragma bank 0
u16 result;
void main(){result=__rle_decode_vram(0x8000,__bankof(encoded),encoded);}

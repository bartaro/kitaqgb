#pragma bank 0
#pragma cgb_palette TONE 1, 2, 3, 4
#pragma cgb_palette TONE #FFFFFF #B0A080 #604830 #000000
#pragma cgb_palette EXISTING 3 4 5 6
__prg_rom u16 EXISTING[4]={7,8,9,10};
u16 result;
void main(){result=TONE[1];result=EXISTING[2];}

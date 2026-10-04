#pragma bank 0
u8 __stackcall down(u8 x){if(x==0)return 0;return down(x-1);}
void main(){down(3);}

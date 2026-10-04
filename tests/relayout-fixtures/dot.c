#include "physics3d.c"
#pragma bank 0
__location(0xC700) u16 result[64];
KQWorld3D w; KQBody3D b[2];
void main(){u8 i;for(i=0;i<64;i++)result[i]=0;result[0]=kq3d_dot_q8_8(-257,257,1,64,-64,127);result[1]=kq3d_dot_q8_8(0,0,0,-128,127,1);result[2]=kq3d_dot_q8_8(256,256,256,127,127,127);result[63]=0xA55A;while(1){}}
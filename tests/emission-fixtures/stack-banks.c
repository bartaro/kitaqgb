#pragma bank 2
u16 __stackcall remote(u8 a,u16 b){return (u16)a+b;}
#pragma bank 1
u8 __stackcall remote_byte(u8 x){return (u8)remote(x,257);}
#pragma bank 0
u16 result;
void main(){result=remote(3,400);result=remote_byte(7);}

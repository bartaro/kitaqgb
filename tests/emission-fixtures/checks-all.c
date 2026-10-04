#pragma bank 1
u8 data[16];u8 src[8];u8 n;u16 out;
u8 byte(u8 x){return data[x];}
u16 __stackcall word(u8 x,u16 y){return y+(u16)byte(x);}
void main(){n=4;__memset(data,7,n);__memcpy(data,src,n);out=word(n,16);__assert(out==16);__unsafe{out=data[n];}}

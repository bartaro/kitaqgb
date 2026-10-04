#pragma bank 0
u8 fixed(u8 x){return x+1;}
#pragma bank 2
u8 remote(u8 x){return x+2;}
#pragma bank 1
u8 byte(u8 x){return x+3;}
u16 word(u16 x){return x+4;}
u16 __stackcall stacked(u8 x,u16 y){return y+x;}
void empty(){}
typedef u8 (*Callback)(u8);
Callback fn;
u8 a;u16 w;
void main(){a=byte(5);w=word(8);w=stacked(a,w);__farcall(1,empty);a=remote(a);a=fixed(a);fn=fixed;a=fn(a);__unsafe{a=byte(a);a=fn(a);}}

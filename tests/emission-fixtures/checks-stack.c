#pragma bank 1
u8 a;u16 w;
u8 byte(u8 p){u16 x;x=(u16)p*31;return (u8)(x/3);}
u16 word(u16 p){return p*257;}
u16 __stackcall stack(u8 p,u16 q){q=q+p;return word(q);}
__unsafe u8 exempt(u8 p){return byte(p);}
void main(){a=byte(9);w=word(258);w=stack(a,w);a=exempt(a);__unsafe{w=stack(2,3);}}

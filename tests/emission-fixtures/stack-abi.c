#pragma bank 0
u16 __stackcall sum(u8 a,u16 b,s8 c){return (u16)a+b+(u16)c;}
u8 __stackcall modified(u8 a,u8 b){a=a+2;b++;return a+b;}
u16 __stackcall mixed(u8 a,u16 b){a++;return a+b;}
u8 __stackcall nested(u8 a){return modified(a,4);}
void __stackcall no_args(){__rng_seed(3);}
u16 result;
void main(){result=sum(3,257,-4);result=modified(3,4);result=mixed(3,400);result=nested(3);no_args();}

#pragma bank 0
u8 state;
u8 __a(u8 n){return n+1;}
u8 __b(u8 n){return n+2;}
u8 __c(u8 n){return n+3;}
u8 __d(u8 n){return n+4;}
u8 __e(u8 n){return n+5;}
u8 __f(u8 n){return n+6;}
u8 __g(u8 n){return n+7;}
u8 __h(u8 n){return n+8;}
u8 ordinary(u8 n){return n+9;}
void __io(){*(u8*)0xFF40=0;}
void __viaio(){__io();}
void __interrupt(){__asm{DI}}
void main(){
state=__a(state);
state=__a(state);
state=__a(state);
state=__a(state);
state=__b(state);
state=__b(state);
state=__b(state);
state=__c(state);
state=__c(state);
state=__d(state);
state=__d(state);
state=__e(state);
state=__e(state);
state=__f(state);
state=__f(state);
state=__g(state);
state=__g(state);
state=__h(state);
state=__h(state);
state=ordinary(state);
state=ordinary(state);
state=ordinary(state);
__scroll_bg_x_set(4);__scroll_split_reset();
__io();__io();__viaio();__viaio();__interrupt();__interrupt();
}

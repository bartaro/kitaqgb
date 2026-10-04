#pragma bank 0
struct Item {u8 a;u16 b;};
struct Item value;struct Item other;u16 result;u8 output;
inline u8 choose(u8 x){if(x)return x+1;return 7;}
inline u16 widen(u16 x,u8 y){u16 z;z=x+y;return z;}
inline void store(u8 x){if(x){output=x;return;}output=9;}
inline struct Item make_inline(u8 a,u16 b){struct Item x;x.a=a;x.b=b;return x;}
inline struct Item copy_inline(struct Item x){return x;}
inline u16 read_inline(struct Item x){return widen(x.b,x.a);}
void main(){output=choose(0);result=widen(257,3);store(4);value=make_inline(5,600);other=copy_inline(value);result=read_inline(other);}

#pragma bank 0
struct Item {u8 a;u16 b;};
struct Tiny {u8 a;};
struct Pair {u16 a;};
struct Nested {struct Item item;};
struct Item value;struct Item other;struct Tiny tiny;struct Pair pair;struct Nested nested;
u16 result;
u16 read(struct Item x){return x.b+x.a;}
struct Item make(u8 a,u16 b){struct Item x;x.a=a;x.b=b;return x;}
struct Item copy(struct Item x){return x;}
struct Tiny gettiny(){return tiny;}
struct Pair getpair(){return pair;}
u16 stack_read(struct Item x){return x.b+x.a;}
struct Item stack_make(u8 a,u16 b){struct Item x;x.a=a;x.b=b;return x;}
void main(){
    value=make(3,257);other=copy(value);result=read(value);
    tiny=gettiny();pair=getpair();
    result=stack_read(value);other=stack_make(5,600);
    nested.item.a=7;nested.item.b=300;
}

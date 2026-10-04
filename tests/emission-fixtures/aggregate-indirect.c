struct Item {u8 a;u16 b;};
typedef struct Item (*Maker)();
typedef struct Item (*MakerArg)(u8);
typedef u16 (*Scalar)(u8);
struct Item value;struct Item other;Maker maker;MakerArg makerarg;Scalar scalar;u16 result;u8 bank;
struct Item named(){return value;}
struct Item named_arg(u8 a){value.a=a;return value;}
u16 scalar_fn(u8 a){return a+256;}
#pragma bank 2
struct Item banked(){return value;}
#pragma bank 0
void main(){maker=named;makerarg=named_arg;scalar=scalar_fn;other=maker();other=makerarg(5);result=scalar(7);__farcall(2,banked);bank=2;__farcall(bank,banked);}

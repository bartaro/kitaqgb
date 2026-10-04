#pragma bank 0
__location(0xC700) u16 results[12];
__location(0xC718) u8 done;
struct Item {u8 a;u16 b;};
struct Large {u8 bytes[260];};
typedef struct Item (*Maker)();
typedef struct Item (*MakerArg)(u8);
struct Item value;struct Item other;struct Large large_a;struct Large large_b;
Maker maker;MakerArg makerarg;u8 bytes[4];u16 words[4];s8 signed_byte;
struct Item make(u8 a,u16 b){struct Item item;item.a=a;item.b=b;return item;}
u16 read(struct Item item){return item.a+item.b;}
inline struct Item inline_make(u8 a,u16 b){struct Item item;item.a=a;item.b=b;return item;}
struct Item get_value(){return value;}
struct Item get_arg(u8 a){value.a=a;return value;}
u16 __stackcall stack_read(struct Item item){return item.a+item.b;}
#pragma bank 2
struct Item remote_make(){struct Item item;item=make(8,610);return item;}
#pragma bank 0
void main(){
    value=make(3,257);results[0]=read(value);
    value=inline_make(5,600);results[1]=read(value);
    maker=get_value;other=maker();results[2]=other.b;
    makerarg=get_arg;other=makerarg(7);results[3]=other.a;
    results[4]=stack_read(other);other=remote_make();results[5]=other.b;
    __memset(&large_a,0,sizeof(struct Large));large_a.bytes[259]=9;large_b=large_a;results[6]=large_b.bytes[259];
    bytes[1]=10;results[7]=bytes[1]++;words[1]=300;results[8]=++words[1];
    other.b=600;results[9]=other.b--;results[10]=(u16)(words[1]=700);
    signed_byte=-2;results[11]=signed_byte++;
    done=165;while(1){}
}

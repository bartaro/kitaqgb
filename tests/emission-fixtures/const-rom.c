#pragma bank 1
extern const u8 byte;
const u8 byte=7;
const s8 signed_byte=-5;
const u16 word=0x3456;
const s16 signed_word=-300;
constexpr u8 compile_time=3;
#pragma bank 0
u8 result;
u16 wide;
const u8 *pointer;
void main(){result=byte;result=signed_byte;wide=word;wide=signed_word;pointer=&byte;result=*pointer;result=compile_time;}

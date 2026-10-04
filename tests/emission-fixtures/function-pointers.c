#pragma bank 0
u8 add(u8 value){return value+3;}
u16 number(){return 0x3456;}
typedef u8 (*ByteFunction)(u8);
typedef u16 (*WordFunction)();
ByteFunction pointer;
WordFunction pointer2;
u16 result;
void main(){pointer=add;pointer2=&number;result=pointer(8);result=pointer2();}

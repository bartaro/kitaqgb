#pragma bank 3
__prg_rom u8 table[4]={1,2,3,4};
u16 number(){return 0x1234;}
#pragma bank 0
typedef u16 (*Callback)();
u8 bytes[8];u8 bank;u16 result;Callback pointer;
void main(){bank=3;result=__farcall(3,number);result=__farcall(__bankof(number),number);result=__farcall(bank,number);pointer=number;__farcall_ptr(bank,pointer);
__far_memcpy(bytes,__bankof(table),table,4);__farmemcpy(bytes,bank,table,4);
result=__farpeek8(bank,table);result=__farpeek16(bank,table);result=__bankof(number);}

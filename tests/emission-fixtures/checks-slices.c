#pragma bank 1
u8 data[8];u8 *pointer;u8 i;u16 len;u8 value;
void main(){pointer=data;len=8;value=__slice(pointer,len)[i];value=__slice(pointer,8)[3];__slice(pointer,len)[i]++;__unsafe{value=__slice(pointer,len)[i];}}

extern u8 numbers[];
extern u8 scalar;
extern u8 scalar;
extern __hram u8 scalar;
extern u8 table[];
u8 numbers[3];
u8 scalar;
__prg_rom u8 table[3]={1,2,3};
void main(){scalar=numbers[0];scalar=table[1];}

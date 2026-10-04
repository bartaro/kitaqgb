struct Item {u8 a;u16 b;};
__prg_rom u8 bytes[3]={1,2,3};
__prg_rom struct Item items[1]={{4,0x5678}};
u8 *p;
void main() {p=bytes; p=&bytes[1];}

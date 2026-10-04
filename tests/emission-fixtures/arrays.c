struct Item {u8 a;u16 b;};
u8 bytes[4]; u16 words[4]; struct Item items[4];
__prg_rom u8 table[4]={2,4,6,8};
u8 i; u8 b; u16 w; struct Item *p;
void main() {
i=1;bytes[i]=3;words[i]=0x1234;b=bytes[i];w=words[i];b=table[2];b=table[i];
items[i].a=b;items[i].b=w;b=items[i].a;w=items[i].b;
p=&items[i];p->a=5;p->b=0xABCD;b=p->a;w=p->b;
*(u8*)0xFF80=b;*(u16*)0xC800=w;
}

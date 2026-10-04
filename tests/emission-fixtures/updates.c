#pragma bank 0
struct Item {u8 a;u16 b;};
u8 bytes[8];u16 words[8];struct Item items[3];u8 i;u8 x;u16 y;u8 *p;u16 *q;struct Item *s;s8 signed_byte;u16 result;
void main(){
    p=bytes;q=words;s=items;i=1;bytes[i]++;words[i]--;items[i].a++;items[i].b--;(*p)++;(*q)--;s->a++;s->b--;
    x=x++;x=++bytes[i];x=items[i].a--;y=words[i]++;y=--items[i].b;result=signed_byte++;p++;q--;s++;
    x=(bytes[i]=7);x=(words[i]=600);x=(*p=8);x=(*q=700);x=(y=i);x=(y=500);
}

#pragma bank 1
struct Item {u8 a;u16 b;};
u8 array[8];u8 big[320];struct Item items[3];u8 i;u16 j;s16 negative;__safe_index u8 safe;u8 value;u16 word;
__unsafe void bypass(){value=array[i];}
void main(){value=array[i];array[i]=value;big[j]++;value=array[negative];word=big[j];value=array[safe];value=array[3];items[i].a=value;word=items[i].b;__unsafe{value=array[i];array[i]++;}bypass();}

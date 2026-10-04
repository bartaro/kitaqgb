#pragma bank 0
u8 kept(u8 x) {return x;}
#pragma bank 1
void main() {kept(9);}

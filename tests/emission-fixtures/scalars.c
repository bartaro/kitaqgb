__hram u8 h; u8 b; u16 w;
u8 byte(u8 x) { return x + 5; }
u16 word(u16 x) { return x ^ 0xA55A; }
void main() { h=9; b=byte(h); w=word(0x1234); w=w+257; h=b&3; b=h|7; b=b^h; }

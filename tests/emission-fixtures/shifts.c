u8 b; s8 c; u16 w; s16 s;
void main() {b=129;c=-3;w=0x1234;s=-300;
b=b<<2;b=c>>1;b=w>>8;w=w<<8;w=w<<9;w=w<<16;
w=w>>8;w=w>>15;w=s>>8;w=s>>18;w=(u16)(c>>2);w=(u16)b;
}

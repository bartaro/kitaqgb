u8 a; u8 b; s8 c; u16 x;
void main() {
a=3; b=5; c=-3; x=258;
if(a<b) a=7; else a=8;
if(c>=0) b=1; else if(x==0) b=2; else b=3;
b=(a==b); a=(a>b); b=(c<0); a=(x>=256);
if(a && b || !c) b=10;
a=a?5:6;
}

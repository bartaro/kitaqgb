u8 a; u8 b;
void main() {
a=0;
for(a=0;a<5;a++) {if(a==2) continue; if(a==4) break; b=b+a;}
do {b--; if(b==3) continue;} while(b>0);
for(a=1;0;a++) b=99;
}

u8 a;u8 b;u16 count;
void main(){a=5;b=0;
switch(a){case 5:b=1;break;case 2:b=2;fallthrough;case 7:b=3;break;default:b=4;}
switch(a){default:b=8;break;}
for(a=5;a!=0;a--)b=b+1;
for(count=2;count>0;count--)b=b+1;
}

#pragma bank 0
u8 __aa(){return 1;}
u8 __Aa(){return 2;}
u8 __a_a(){return 3;}
u8 __a0(){return 4;}
u8 __A0(){return 5;}
u8 __apple(){return 6;}
u8 __Apple(){return 7;}
u8 result;
void main(){result=__aa();result=__aa();result=__Aa();result=__Aa();result=__a_a();result=__a_a();result=__a0();result=__a0();result=__A0();result=__A0();result=__apple();result=__apple();result=__Apple();result=__Apple();}

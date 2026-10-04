const u8 MASK=15;
const u8 VALUE=3;
u8 result;
void main(){
    if((VALUE&MASK)<4)result=1;else result=2;
    if((VALUE&MASK)>8)result=3;
    result=(VALUE&MASK)/8;
    result=(VALUE&MASK)%8;
}

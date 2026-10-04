#pragma bank 2
u8 Link_Begin(){return 1;}
u8 Link4_Begin(){return 2;}
u8 Alpha(){return 3;}
u8 alpha(){return 4;}
u8 __helper(){return 5;}
#pragma bank 0
u8 result;
void main(){result=Link4_Begin();result=Link_Begin();result=Alpha();result=alpha();result=__helper();result=__farcall(result,Link_Begin);result=__farcall(result,Link4_Begin);result=__farcall(result,alpha);result=__farcall(result,Alpha);}

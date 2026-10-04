#pragma bank 2
u8 remote(u8 a){return a+7;}
#pragma bank 1
u16 remote_word(u16 a){return a+257;}
#pragma bank 0
u8 byte_result;u16 word_result;
void main(){byte_result=remote(8);word_result=remote_word(0x1234);__bankswitch(3);}

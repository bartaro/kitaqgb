#pragma bank 1
const u8 byte=7;
const s8 signed_byte=-5;
const u16 word=0x3456;
const s16 signed_word=-300;
#pragma bank 0
__location(0xC700) u16 results[12];
__location(0xC718) u8 done;
const u8 *pointer;
const u16 *word_pointer;
const s16 *signed_word_pointer;
u8 i;
void main(){
__bankswitch(1);
for(i=0;i<12;i++)results[i]=0;
results[0]=byte;results[1]=(s16)signed_byte;
results[2]=word;results[3]=signed_word;
pointer=&byte;results[4]=*pointer;
word_pointer=&word;results[5]=*word_pointer;
signed_word_pointer=&signed_word;results[6]=*signed_word_pointer;
done=165;while(1){}
}

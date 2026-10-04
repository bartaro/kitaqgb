#pragma bank 1
__location(0xC700) u16 results[12];
__location(0xC718) u8 done;
u8 data[8];u8 i;
void main(){for(i=0;i<12;i++)results[i]=0;i=2;data[i]=42;results[0]=data[i];__assert(results[0]==42,77);
/* failure injection */
done=165;while(1){}}

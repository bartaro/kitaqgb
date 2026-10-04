#pragma bank 1
__location(0xC700) u16 results[12];
__location(0xC718) u8 done;
u8 data[8];u8 src[8];u8 n;u8 i;
u8 load(u8 x){return data[x];}
u16 __stackcall add(u8 x,u16 y){return y+(u16)load(x);}
void main(){
for(i=0;i<12;i++)results[i]=0;
n=8;__memset(data,7,n);__memset_small(src,5,n);__memcpy(data,src,n);
results[0]=add(3,37);__assert(results[0]==42,77);
/* failure injection */
done=165;while(1){}
}

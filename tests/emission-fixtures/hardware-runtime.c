#pragma bank 2
u16 __stackcall remote(u8 a,u16 b){return (u16)a+b;}
#pragma bank 1
u8 __stackcall remote_byte(u8 x){return (u8)remote(x,257);}
#pragma bank 0
__location(0xC700) u16 results[12];
__location(0xC718) u8 done;
__location(0xC800) u8 shadow[160];
u8 src[32];u8 dst[32];
void main(){
    __memset(dst,0,32);__memset(src,60,32);
    __bit_set(dst,9);results[0]=__bit_test(dst,9);
    __bit_toggle(dst,9);results[1]=__bit_test(dst,9);
    __memcpy(dst,src,19);results[2]=dst[18];results[3]=dst[19];
    results[4]=__map_index(3,5,19);results[5]=__manhattan(4,6,8,1);
    results[6]=__mul8x8_hi(200,200);
    results[7]=(u16)__sdot2_q8_8(-1024,512,64,-32);
    results[8]=(u16)__sdot2_q1_7(-3,0,64,0);
    results[9]=remote(3,400);results[10]=remote_byte(7);
    __memset(shadow,0,160);shadow[0]=32;shadow[1]=40;shadow[2]=1;
    __oam_dma(shadow);
    *(u8*)0xFF40=0;
    __vram_memset(0x8000,5,9);
    results[11]=*(u8*)0x8008;
    done=165;
    while(1){}
}

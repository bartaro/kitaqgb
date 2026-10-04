#pragma bank 2
__prg_rom u8 encoded[5]={3,7,2,9,0};
#pragma bank 3
__prg_rom u8 payload[4]={1,2,3,4};
u16 number(){return 0x1234;}
void callback(){results[2]=77;}
#pragma bank 0
__location(0xC700) u16 results[12];
__location(0xC718) u8 done;
__location(0xC740) u8 copied[8];
typedef void (*Callback)();
Callback pointer;
u8 bank;
void main(){
    __memset(results,0,24);__memset(copied,0,8);
    bank=3;
    results[0]=__farcall(__bankof(number),number);
    results[1]=__farcall(bank,number);
    pointer=callback;__farcall_ptr(bank,pointer);
    __far_memcpy(copied,__bankof(payload),payload,4);
    results[3]=__farpeek8(__bankof(payload),payload);
    results[4]=__farpeek16(__bankof(payload),payload);
    results[5]=__cgb_is_cgb();
    results[6]=__rom_bank;
    __scroll_bg_set(1,2);__scroll_bg_set_buffered(11,12);
    results[7]=__scroll_bg_x_get();
    __scroll_flush();results[8]=__scroll_bg_x_get();results[9]=__scroll_bg_y_get();
    *(u8*)0xFF40=0;
    results[10]=__rle_decode_vram(0x8000,__bankof(encoded),encoded);
    results[11]=copied[3];
    done=165;
    while(1){}
}

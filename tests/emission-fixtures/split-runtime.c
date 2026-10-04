#pragma bank 0
__location(0xC700) u16 results[12];
__location(0xC718) u8 done;
u8 i;
void main(){
    __asm { DI }
    *(u8*)0xFF40=0;
    __scroll_bg_set(1,2);
    __scroll_split_reset();
    for(i=0;i<9;i++) __scroll_split_push(i*16,i+3,i+4);
    results[0]=*(u8*)0xD00A;results[1]=*(u8*)0xD03D;
    __scroll_split_reset();results[2]=*(u8*)0xD00A;
    __scroll_split_push(0,3,4);__scroll_split_push(32,5,6);__scroll_split_push(64,7,8);
    __scroll_split_commit();
    results[3]=*(u8*)0xD00B;results[4]=*(u8*)0xFF45;
    *(u8*)0xC29C=0;*(u8*)0xFF0F=0;*(u8*)0xFFFF=3;*(u8*)0xFF40=0x91;
    __asm { EI }
    while(*(u8*)0xC29C==0){}
    results[5]=*(u8*)0xC29C;
    while(*(u8*)0xFF44!=100){}
    __asm { DI }
    results[6]=*(u8*)0xFF43;results[7]=*(u8*)0xFF42;
    results[8]=*(u8*)0xD00C;results[9]=*(u8*)0xFF45;
    results[10]=__scroll_bg_x_get();results[11]=*(u8*)0xC29C;
    *(u8*)0xFF40=0;done=165;while(1){}
}

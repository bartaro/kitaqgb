#pragma bank 0
__location(0xC700) u16 results[12];
__location(0xC718) u8 done;
__location(0xC800) u8 buffer[128];
__location(0xC880) u8 attributes[128];
__prg_rom u8 source[9]={10,11,12,13,14,15,16,17,18};
void main(){
    *(u8*)0xFF40=0;
    __vram_memset(0x9800,0,1024);
    __settile(0,0,6);results[0]=*(u8*)0x9800;
    __settile(32,0,99);results[1]=*(u8*)0x9800;
    __settile_row(0,1,source,3);results[2]=*(u8*)0x9822;
    __settile_col(4,1,source,3);results[3]=*(u8*)0x9864;
    __settile_rect(8,1,2,2,7);results[4]=*(u8*)0x9849;
    __settilemap_rect(0x9800,12,1,2,2,source);results[5]=*(u8*)0x984D;
    __settilebg16_buf(buffer,1,1,30);
    __settilebg16_flush(buffer,1,2,2);results[6]=*(u8*)0x9863;
    __settilebg16cgb_buf(buffer,attributes,1,1,40,5);
    __settilebg16cgb_flush(buffer,attributes,1,2,2);results[7]=*(u8*)0x9863;
    __settileatcgb_unsafe(0x9800,0,0,60,7);results[8]=*(u8*)0x9800;
    __cgb_safe_set_vbk(1);results[9]=*(u8*)0x9800;results[10]=*(u8*)0x9863;
    __cgb_safe_set_vbk(0);results[11]=__cgb_is_cgb();
    done=165;while(1){}
}

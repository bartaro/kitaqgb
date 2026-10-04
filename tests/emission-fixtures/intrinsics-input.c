u8 a;u8 token;u16 w;u8 state[5];
void main(){
__wait_vblank();__wait_ly(20);
__rng_seed(0x1234);a=__rng8();w=(u16)__rng8();
token=__critical_enter();__critical_leave(token);
a=__readpad();a=__readpaddir();a=__readpadbtn();w=__readpadex(a);
w=__getbgmapbase();w=__getwinmapbase();
a=__sram_read8(0xA000);__sram_write8(0xA001,a);
__padrep_init(state,10,2);__padrep_reset(state);
a=__padrep_lr(state,a,a);a=__padrep_down(state,a,a);a=__padrep_mask(state,a,a,4);
}

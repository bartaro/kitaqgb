#pragma rom_cgb cgb_only
#pragma bank 0
u8 value;
void main(){value=__cgb_is_cgb();__cgb_safe_set_vbk(value);value=__svbk_get();value=__svbk_set(2);value=__svbk_set(value);}

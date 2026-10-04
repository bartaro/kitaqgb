u8 source[32];u8 b;u16 n;
void main(){
__vram_memcpy(0x8000,source,0);__vram_memset(0x8000,b,0);
__vram_memcpy(0x8000,source,3);__vram_memset(0x8000,b,3);
__vram_memcpy_unsafe(0x8000,source,3);__vram_memset_unsafe(0x8000,b,3);
__vram_memcpy(0x8000,source,9);__vram_memset(0x8000,b,9);
__vram_memcpy_unsafe(0x8000,source,n);__vram_memset_unsafe(0x8000,b,n);
__vram_copy_hblank(0x8000,source,b);__fill_tilemap(0x9800,b,n);
}

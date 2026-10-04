u8 x;u8 y;u8 tile;u8 attr;u8 count;u16 dest;u16 src;u16 attrs;
void main(){
    __settileattr(x,y,attr); __settileattr_unsafe(1,2,3);
    __settilecgb(x,y,tile,attr); __settilecgb_unsafe(1,2,3,4);
    __settileatattr(dest,x,y,attr); __settileatattr_unsafe(0x9800,1,2,3);
    __settileatcgb(dest,x,y,tile,attr); __settileatcgb_unsafe(0x9c00,1,2,3,4);
    __settilewinattr(x,y,attr); __settilewinattr_unsafe(1,2,3);
    __settilewincgb(x,y,tile,attr); __settilewincgb_unsafe(1,2,3,4);
    __settilebgattr(x,y,attr); __settilebgattr_unsafe(1,2,3);
    __settilebgcgb(x,y,tile,attr); __settilebgcgb_unsafe(1,2,3,4);
    __settileattr_bulk(dest,src,count); __settileattr_bulk_fast(dest,src,count);
    __settileattr_bulk(dest,src,3); __settileattr_bulk_fast(dest,src,0);
    __settilecgb_bulk(dest,src,attrs,count); __settilecgb_bulk_fast(dest,src,attrs,count);
    __settilecgb_bulk(dest,src,attrs,3); __settilecgb_bulk_fast(dest,src,attrs,0);
    __settilebg16_buf(dest,x,y,tile); __settilebg16cgb_buf(dest,attrs,x,y,tile,attr);
    __settilebg16_flush(src,y,x,count); __settilebg16_flush(src,1,2,3);
    __settilebg16cgb_flush(src,attrs,y,x,count); __settilebg16cgb_flush(src,attrs,1,2,3);
}

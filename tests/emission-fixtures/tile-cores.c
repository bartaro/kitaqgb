u8 x;u8 y;u8 tile;u8 n;
u16 dest;u16 src;
void main() {
    __settile(x,y,tile); __settile_unsafe(1,2,3); __settile_fast(31,31,4);
    __settileat(dest,x,y,tile); __settileat_unsafe(0x9c00,33,1,5);
    __settilewin(x,y,tile); __settilewin_unsafe(2,3,4);
    __settilebg(x,y,tile); __settilebg_unsafe(4,5,6);
    __settile_bulk(dest,src,n); __settile_bulk_fast(dest,src,n);
    __settile_bulk(dest,src,3); __settile_bulk_fast(dest,src,0);
    __settile_xy(1,2,3);
}

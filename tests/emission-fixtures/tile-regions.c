u8 x;u8 y;u8 w;u8 h;u8 tile;u16 base;u16 src;
void main(){
    __settile_rect(x,y,w,h,tile); __settile_rect(1,2,3,4,5);
    __settile_row(x,y,src,w); __settile_row(1,2,src,0); __settile_row(1,2,src,3);
    __settile_col(x,y,src,h); __settile_col(1,2,src,0); __settile_col(1,2,src,3);
    __settilemap_rect(base,x,y,w,h,src);
    __settilemap_rect(base,x,y,3,1,src); __settilemap_rect(0x9800,1,y,w,1,src);
    __settilemap_rect(base,x,y,1,3,src); __settilemap_rect(0x9c00,x,2,1,h,src);
    __settilemap_rect(0x9800,0,0,2,2,src);
}

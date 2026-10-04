#pragma bank 1
u8 dst[16];u8 src[8];u8 n;u16 count;
void fill(){__memset(dst,7,n);__memset_small(dst,8,n);__memset(dst,9,count);}
void copy(){__memcpy(dst,src,n);__memcpy_small(dst,src,n);__memcpy(dst,src,count);}
void faildst(){__memset(dst,7,17);}
void failsrc(){__memcpy(dst,src,9);}
void equal(){__memset_small(dst,7,272);__memcpy(dst,src,8);}
void main(){n=4;count=8;fill();copy();__unsafe{__memcpy(dst,src,count);}equal();}

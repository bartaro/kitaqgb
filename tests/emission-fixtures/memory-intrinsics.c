u8 dst[512];u8 src[512];u8 b;u16 w;
void main(){
__memcpy(dst,src,0);__memset(dst,7,0);
__memcpy(dst,src,3);__memset(dst,b,3);
__memcpy(dst,src,19);__memset(dst,b,19);
__memcpy(dst,src,97);__memset(dst,b,97);
__memcpy(dst,src,256);__memset(dst,b,256);
__memcpy_small(dst,src,259);__memset_small(dst,b,259);
__memcpy_small(dst,src,b);__memset_small(dst,b,b);
__memcpy(dst,src,w);__memset(dst,b,w);
__copy16(dst,src);__copy32(dst,src);
}

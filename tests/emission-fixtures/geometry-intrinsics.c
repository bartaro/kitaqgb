u8 a;u8 b;u8 bits[32];u16 result;
void main(){
result=__tile_addr(0x9800,1,2);result=__tile_addr(0x9800,a,2);result=__tile_addr(0x9800,1,b);result=__tile_addr(0x9800,a,b);
result=__map_index(1,2,32);result=__map_index(a,b,32);result=__map_index(0,b,256);result=__map_index(a,b,a);
a=__xy_in_rect(1,2,0,0,4,4);a=__xy_in_rect(a,b,1,2,a,b);
a=__manhattan(3,4,6,8);a=__manhattan(a,b,b,a);
__bit_set(bits,9);__bit_clear(bits,result);__bit_toggle(bits,result);a=__bit_test(bits,result);
}

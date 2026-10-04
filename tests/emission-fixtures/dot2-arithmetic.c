u8 a;u8 b;u16 x;u16 y;s16 z;
void main(){a=__mul8x8_hi(a,b);
x=__dot2_q8_8(x,y,0,0);x=__dot2_q8_8(x,y,0,a);x=__dot2_q8_8(x,y,a,0);x=__dot2_q8_8(x,y,a,b);
z=__sdot2_q8_8(z,z,0,0);z=__sdot2_q8_8(z,z,0,-32);z=__sdot2_q8_8(z,z,-64,0);z=__sdot2_q8_8(z,z,a,b);
z=__sdot2_q1_7(z,z,0,0);z=__sdot2_q1_7(z,z,32,-64);z=__sdot2_q1_7(z,z,a,b);
}

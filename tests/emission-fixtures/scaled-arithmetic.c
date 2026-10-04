u8 a;u8 b;s8 c;s8 d;u16 x;u16 y;s16 z;
void main(){a=5;b=7;c=-3;d=9;x=0x1234;y=0x4321;z=-2003;
x=__mul16x8(x,a);z=__smul16x8(z,c);z=__smul16x8_q1_7(z,64);
x=__mac16(x,y,b);z=__smac16(z,z,c);z=__smac16_q1_7(z,z,d);
x=__dot3_q8_8(x,a,y,32,x,0);
z=__sdot3_q8_8(z,c,z,-64,z,0);
z=__sdot3_q1_7(z,d,z,-32,z,0);
}

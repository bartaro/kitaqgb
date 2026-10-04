#pragma bank 0

u8 arith_done;
u8 arith_results[50];

void put16(u8 index, u16 value) {
    arith_results[index] = (u8)value;
    arith_results[(u8)(index + 1)] = (u8)(value >> 8);
}

void main() {
    u8 ua;
    u8 ub;
    u16 ux;
    u16 uy;
    s8 sa;
    s8 sb;
    s16 sx;
    s16 sy;

    ua = 200;
    ub = 13;
    arith_results[0] = (u8)(ua + ub);
    arith_results[1] = (u8)(ua - ub);
    arith_results[2] = (u8)((u8)13 * (u8)19);
    arith_results[3] = (u8)(ua / ub);
    arith_results[4] = (u8)(ua % ub);
    arith_results[5] = (u8)(ub << 3);
    arith_results[6] = (u8)(ua >> 3);
    arith_results[7] = (u8)(ua & (u8)0x5A);
    arith_results[8] = (u8)(ua | (u8)0x12);
    arith_results[9] = (u8)(ua ^ (u8)0xFF);

    ux = 50000;
    uy = 321;
    put16(10, ux + uy);
    put16(12, ux - uy);
    put16(14, (u16)300 * (u16)123);
    put16(16, ux / uy);
    put16(18, ux % uy);
    put16(20, (u16)(0x1234 << 3));
    put16(22, (u16)(0xF123 >> 4));
    put16(24, (u16)(0x1234 & 0x0FF0));
    put16(26, (u16)(0x1234 | 0x00C0));
    put16(28, (u16)(0x1234 ^ 0xFFFF));

    sa = -7;
    sb = 3;
    arith_results[30] = (u8)(sa + sb);
    arith_results[31] = (u8)(sa - sb);

    sx = -1234;
    sy = 17;
    put16(32, (u16)(sx / sy));
    put16(34, (u16)(sx % sy));
    put16(36, (u16)((s16)-16 >> 2));

    if (sa < sb) {
        arith_results[38] = 1;
    } else {
        arith_results[38] = 2;
    }
    if (sx > sy) {
        arith_results[39] = 1;
    } else {
        arith_results[39] = 2;
    }
    if ((u8)250 > (u8)4) {
        arith_results[40] = 1;
    } else {
        arith_results[40] = 2;
    }
    if ((s8)-1 > (s8)1) {
        arith_results[41] = 1;
    } else {
        arith_results[41] = 2;
    }
    if ((u16)65535 > (u16)1) {
        arith_results[42] = 1;
    } else {
        arith_results[42] = 2;
    }
    if ((s16)-1 > (s16)1) {
        arith_results[43] = 1;
    } else {
        arith_results[43] = 2;
    }

    ua = 10;
    ua += 7;
    ua *= 3;
    ua -= 6;
    ua /= 5;
    ua %= 4;
    arith_results[44] = ua;

    ux = 3;
    ux <<= 5;
    ux += 1;
    ux >>= 2;
    put16(45, ux);

    uy = 0x0100;
    put16(48, (u16)~uy);

    arith_done = 0xA5;

    while (1) {
    }
}

#pragma bank 0

s8 sx;
s8 sy;
s16 wx;
s16 wy;
s16 q;
s16 r;

void main() {
    sx = -4;
    sy = 1;
    wx = (s16)sx;
    wy = wx >> 1;

    if (sx < sy) {
        q = 7;
    } else {
        q = 9;
    }

    q = (s16)-13 / (s16)5;
    r = (s16)-13 % (s16)5;

    while (1) {
    }
}

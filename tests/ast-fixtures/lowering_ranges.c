#pragma bank 0
u8 buf[256];
u16 result;
void main() {
    __range(0, 255) u16 i;
    __range(-7, 15) s16 j;
    i = 3;
    j = 9;
    if (i < 16 && i > 4) {
        result = buf[i];
        if (i >= 16) result = 1; else result = 2;
    } else if (!(i < 32)) {
        if (i < 32) result = 3; else result = buf[i];
    }
    if (j >= 0) result = (u16)(j / 8);
    result = (u16)(i / 8 + i % 16 + 4 * i);
    result = buf[(u8)(i & 127)];
    if ((i + 1) > 256) result = 4;
    if (j == 9) result = 5;
    result = buf[j];
    while (1) {}
}

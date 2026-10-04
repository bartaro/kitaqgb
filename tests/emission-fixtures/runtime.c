#pragma bank 0
__location(0xC700) u8 result0;
__location(0xC701) u8 result1;
__location(0xC702) u8 result2;
__location(0xC703) u8 result3;
__location(0xC704) u16 result4;
__location(0xC706) u8 done;
u8 plus(u8 x) { return x + 5; }
u16 word(u16 x) { return x ^ 0xA55A; }
void main() {
    u8 i;
    u8 sum;
    s8 signed_byte;
    sum = 0;
    for (i = 0; i < 5; i++) {
        if (i == 2) continue;
        if (i == 4) break;
        sum = sum + i;
    }
    result0 = plus(9);
    result1 = sum;
    do { sum--; if (sum == 3) continue; } while (sum > 0);
    result2 = sum;
    signed_byte = -3;
    result3 = signed_byte < 0;
    result4 = word(0x1234);
    done = 0xA5;
    while (1) {}
}

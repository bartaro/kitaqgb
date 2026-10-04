#pragma bank 0

define u8 CONST_A = 5;

struct Pair {
    u8 lo;
    u16 hi;
};

__prg_rom u8 ROM8[8] = { 1, 3, 5, 7, 9, 11, 13, 15 };
__wram struct Pair PAIRS[4];
__wram u8 BUF[32];
__wram u8 G0;
__wram u8 G1;
__wram u16 G2;

u8 mix5(u8 a, u8 b, u8 c, u8 d, u8 e) {
    return (u8)(a ^ b ^ c ^ d ^ e);
}

void main() {
    u8 i = 1;
    u8 j = 2;
    u8 k = 3;
    u16 t = 0;

    static_assert(sizeof(u8) == 1, "u8 size");
    static_assert(sizeof(struct Pair) >= 3, "pair size");

    PAIRS[0].lo = (u8)((i + (j * k)) ^ (CONST_A << 1));
    PAIRS[0].lo ^= (u8)(ROM8[(u8)(k - 1)] & 0x0F);
    BUF[(u8)(i + (j << 1))] = (u8)(PAIRS[0].lo + ROM8[(u8)(k - 1)]);
    BUF[(u8)(i + 1)] += (u8)(k * 2);

    G0 = (u8)((i++ + ++j) ? (j | k) : (j & k));
    G1 = mix5(i, (u8)(j + 1), BUF[(u8)(k - 1)], (u8)(~i), (u8)(i ? j : k));

    t = (u16)(((u16)BUF[0] << 8) | (u16)BUF[1]);
    G2 = (u16)(t ^ (u16)CONST_A);

    while (1) { }
}

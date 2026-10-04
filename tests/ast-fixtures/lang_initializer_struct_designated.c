struct Pair {
    u8 x;
    u16 w;
    u8 z;
};

const u8 g_auto[] = { 1, 2, 3, 4, };
const u8 g_pad[6] = { 9, 8 };

u8 sink0;
u8 sink1;
u8 sink2;
u8 sink3;
u8 sink4;
u8 sink5;

void main() {
    struct Pair p0 = { 4, 0x1234, 5 };
    struct Pair p1 = { .z = 7, .x = 6, .w = 0x3456 };

    u8 a[] = { 10, 11, 12 };
    u8 b[4] = { 20, 21 };

    struct Pair tbl[2] = { { .x = 8, .w = 0x0102, .z = 9 }, { 1, 0x0203, 4 } };

    sink0 = p0.x;
    sink1 = (u8)p1.w;
    sink2 = p1.z;
    sink3 = a[2];
    sink4 = b[3];
    sink5 = tbl[1].x;

    while (1) {
    }
}

#pragma bank 0

struct Pixel {
    u8 x;
    u8 y;
};

struct Pixel points[16];
u8 sink;

void main() {
    __range(0, 15) u16 i;
    __range(0, 14) u16 j;

    i = 3;
    j = i;

    points[i].x = 1;
    points[i].y = points[j + 1].x;
    (&points[0])->x = points[j].y;
    sink = points[j + 1].x;

    while (1) {
    }
}

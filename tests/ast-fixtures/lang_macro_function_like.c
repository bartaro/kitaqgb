#define INC(x) ((x) + 1)
#define TWICE(x) ((x) + (x))
#define MUL_ADD(a,b,c) (((a) * (b)) + (c))
#define COMPOSE(x,y) (MUL_ADD(INC(x), 2, (y)))

static_assert(INC(2) == 3, "inc");
static_assert(MUL_ADD(2, 3, 4) == 10, "muladd");
static_assert(COMPOSE(2, 1) == 7, "compose");

u8 g1[INC(3)];
u8 g2[TWICE(2)];

u8 sink0;
u16 sink1;

void main() {
    u8 a;
    u16 b;

    a = INC(5);
    b = MUL_ADD(a, 3, 1);

    sink0 = a;
    sink1 = b;

    while (1) {
    }
}

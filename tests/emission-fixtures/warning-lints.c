#pragma bank 0

u8 g_used;
u8 g_unused;
u8 g_buf[16];

u8 helper_unused(u8 x, u8 y) {
    return x;
}

u8 narrow_ret(u16 x) {
    return x;
}

u8 helper_used(u8 x) {
    return x;
}

void main() {
    u8 a;
    u16 wide;
    s16 neg;
    u8* p;
    u8* q;
    u8 unused_local;

    wide = 300;
    a = wide + 1;
    a = helper_used(wide);

    p = &g_buf[0];
    q = p + 1;
    neg = -1;
    q = p + neg;
    wide = p - q;

    g_used = narrow_ret(wide);

    return;
    g_used = 3;
}

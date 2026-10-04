#pragma bank 0
u8 result;
u8 byte_arg(u8 value) { return value; }
void main() {
    u8 i = 0;
    u8 n = 3;
    u16 v = 128;
    for (i = 0; byte_arg((u8)(v >> n)); i++) {
        if (i == 1) continue;
        __unsafe { if (i == 2) continue; }
        switch (i) {
            case 3: continue;
            default: result++;
        }
        while (result) { if (i) continue; break; }
    }
    do { if (i) continue; result++; } while (v >> n);
    result = (u8)((i && (v >> n)) || (i ? (v << n) : (v >> n)));
    if (0) result = 1; else if (1) result = 2; else result = 3;
    for (i = 4; 0; i++) result = 4;
    return;
    result = 5;
tail:
    result = byte_arg((u8)(i + 1));
}

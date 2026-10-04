__wramx u8 automatic_before[256];
__location(0xD180) u8 fixed_block[32];
__wramx u8 automatic_after[256];

void main() {
    automatic_before[0] = 1;
    fixed_block[0] = 2;
    automatic_after[0] = 3;
    while(1) { }
}

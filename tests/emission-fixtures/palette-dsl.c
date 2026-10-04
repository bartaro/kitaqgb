#pragma bank 0
#pragma cgb_palette UITONE #FFFFFF #B0A080 #604830 #000000

__wram u8 G;

void main() {
    G = (u8)(UITONE[0] & 31);
    while (1) { }
}

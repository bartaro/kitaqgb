// Minimal Game Boy serial register declarations for the shared link library.
// Compile this file before link.c and link_packet.c unless your project
// already declares SB/SC/IF/IE in another hardware-register unit.

__location(0xFF01) u8 SB;
__location(0xFF02) u8 SC;
__location(0xFF0F) u8 IF_REG;
__location(0xFFFF) u8 IE_REG;

u8 LinkHw_ReadSB() {
    return SB;
}

u8 LinkHw_ReadSC() {
    return SC;
}

u8 LinkHw_ReadIF() {
    return IF_REG;
}

u8 LinkHw_ReadIE() {
    return IE_REG;
}

void LinkHw_WriteSB(u8 value) {
    SB = value;
}

void LinkHw_WriteSC(u8 value) {
    SC = value;
}

void LinkHw_WriteIF(u8 value) {
    IF_REG = value;
}

void LinkHw_WriteIE(u8 value) {
    IE_REG = value;
}

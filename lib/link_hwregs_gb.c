// Minimal Game Boy serial register declarations for the shared link library.
// Compile this file before link.c and link_packet.c unless your project
// already declares SB/SC/IF/IE in another hardware-register unit.

__location(0xFF01) u8 SB;
__location(0xFF02) u8 SC;
__location(0xFF0F) u8 IF_REG;
__location(0xFFFF) u8 IE_REG;

// Read the serial data register; software decides whether a transfer has completed.
u8 LinkHw_ReadSB() {
    return SB;
}

// Read serial control, including the active-transfer and clock-source bits.
u8 LinkHw_ReadSC() {
    return SC;
}

// Read all interrupt request bits so callers can preserve unrelated requests.
u8 LinkHw_ReadIF() {
    return IF_REG;
}

// Read all interrupt enable bits without changing the CPU interrupt master enable.
u8 LinkHw_ReadIE() {
    return IE_REG;
}

// Replace the serial data byte. Prepare it before arming SC for the next transfer.
void LinkHw_WriteSB(u8 value) {
    SB = value;
}

// Write the full serial control register, allowing the caller to arm or cancel transfer.
void LinkHw_WriteSC(u8 value) {
    SC = value;
}

// Replace the full interrupt request register; mask unrelated bits at the call site.
void LinkHw_WriteIF(u8 value) {
    IF_REG = value;
}

// Replace all interrupt enable bits; the caller preserves sources it does not own.
void LinkHw_WriteIE(u8 value) {
    IE_REG = value;
}

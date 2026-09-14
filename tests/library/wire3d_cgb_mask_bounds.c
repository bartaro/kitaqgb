// Boundary regression for the CGB bitmap mask and full-screen span lists.
// Compile after wire3d_cgb.c: internal state is inspected deliberately here.
// Build from the repository root (one command):
// kitaqgb lib/wire3d_cgb.c tests/library/wire3d_cgb_mask_bounds.c
//   -I lib -I examples --profile=dev --stack-bank=fixed --rst-disable
//   --no-disasm -o mask_bounds.gbc
// In KOKURA, run as CGB for 1800 frames. Expect CGB MASK PASS and 000.
#include "wire3d_cgb.h"
#include "gb_common.h"
#pragma bank 6
u8 probe_results[24];
u8 probe_count;
u8 probe_done;
u8 probe_failed;
u8 probe_case;
u8 probe_row;
u8 probe_col;
u8 probe_expected;
u8 probe_min;
u8 probe_max;
u8 probe_bit;
u16 probe_offset;

// A trace point here measures completion without depending on caller ROM layout.
void probe_mark_done() {
    __asm {
        NOP
        RET
    }
}

// Check one flat, vertical or unit-slope triangle against analytic coverage.
// Normal mode compares every mask byte; full mode compares inclusive span
// endpoints and the empty second slot. Bit 0 reports coverage failure and
// bit 1 reports corruption of the adjacent-state sentinel.
void probe_check(u8 mode,u8 left,u8 right,u8 top,u8 bottom) {
    w3dcgb_full_mode=mode;
    w3dcgb_clear_occlusion_mask_asm();
    if (mode) w3dcgb_full_clear_spans_asm();
    // This byte immediately follows the bitmap; the harness checks the map file.
    w3dcgb_occlusion_active=0x25;
    Wire3DCGB_MarkTriangle2D(left,top,right,top,right,bottom);
    probe_mark_done();
    probe_results[probe_count]=0;
    if (w3dcgb_occlusion_active!=0x25) probe_results[probe_count]=2;
    // Independently derive each expected row; do not reuse the renderer mask writer.
    probe_row=0;
    while (probe_row < (mode ? 144 : 96)) {
        probe_min=left;
        if ((bottom>top)&&(left!=right)&&(probe_row>=top)&&(probe_row<=bottom))
            probe_min=(u8)(left+probe_row-top);
        if (probe_min>0) probe_min--;
        probe_max=right;
        if (probe_max < (mode ? 159 : 127)) probe_max++;
        if (mode) {
            probe_expected=0xFF;
            if ((probe_row>=top)&&(probe_row<=bottom)) probe_expected=probe_min;
            if (w3dcgb_occlusion_mask[probe_row]!=probe_expected)
                probe_results[probe_count]|=1;
            if ((probe_row>=top)&&(probe_row<=bottom)) {
                if (w3dcgb_occlusion_mask[(u16)probe_row+144]!=probe_max)
                    probe_results[probe_count]|=1;
            }
            if (w3dcgb_occlusion_mask[(u16)probe_row+288]!=0xFF)
                probe_results[probe_count]|=1;
        } else {
            probe_col=0;
            while (probe_col<16) {
                probe_expected=0;
                probe_bit=0;
                while (probe_bit<8) {
                    if ((probe_row>=top)&&(probe_row<=bottom)&&
                        ((u8)(probe_col*8+probe_bit)>=probe_min)&&
                        ((u8)(probe_col*8+probe_bit)<=probe_max))
                        probe_expected|=(u8)(0x80>>probe_bit);
                    probe_bit++;
                }
                probe_offset=(u16)probe_row*16+probe_col;
                if (w3dcgb_occlusion_mask[(__safe_index u16)probe_offset]!=probe_expected)
                    probe_results[probe_count]|=1;
                probe_col++;
            }
        }
        probe_row++;
    }
    if (probe_results[probe_count]!=0) probe_failed++;
    probe_count++;
}
// Run eleven 128x96 and nine 160x144 boundary cases, then expose completion
// and failure bytes for emulator watches and show the result on screen.
void main() {
    probe_count=0; probe_done=0; probe_failed=0;
    probe_check(0,0,0,0,0);
    probe_check(0,64,64,48,48);
    probe_check(0,126,126,0,0);
    probe_check(0,127,127,0,0);
    probe_check(0,127,127,48,48);
    probe_check(0,127,127,95,95);
    probe_check(0,0,127,0,0);
    probe_check(0,0,127,95,95);
    probe_check(0,127,127,0,95);
    probe_check(0,120,127,10,17);
    probe_check(0,120,127,88,95);
    probe_check(1,0,0,0,0);
    probe_check(1,64,64,48,48);
    probe_check(1,127,127,95,95);
    probe_check(1,158,158,0,0);
    probe_check(1,159,159,0,0);
    probe_check(1,159,159,143,143);
    probe_check(1,0,159,143,143);
    probe_check(1,159,159,0,143);
    probe_check(1,152,159,136,143);
    probe_done=0xA5;
    m_init();
    // CGB output uses color palette RAM, not the DMG BGP register.
    m_wait(); M_LCDC=0;
    Wire3DCGB_SetPaletteRGB15(32767,22197,10570,0);
    M_LCDC=0x91;
    if (probe_failed==0) m_text(1,2,"CGB MASK PASS");
    else m_text(1,2,"CGB MASK FAIL");
    m_text(1,4,"FAILED CASES");
    m_number(probe_failed);
    while (1) m_wait();
}

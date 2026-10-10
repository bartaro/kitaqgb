// Regression for single-color CGB palette writes. Build from the repository root:
// kitaqgb compile tests/library/cgb_palette_single_color.c -I lib
//   -o palette_probe.gb --cgb=cgb --rst-disable --no-disasm
// Run in CGB mode. At $C100 expect: A55A, 0000, 0084, 0001 (little-endian words).
// This inspects palette RAM with the LCD off; it does not test LCD-on timing.
#include "cgb_palette.c"

__location(0xFF40) u8 probe_lcdc;
__location(0xFF68) u8 probe_bg_index;
__location(0xFF69) u8 probe_bg_data;
__location(0xFF6A) u8 probe_obj_index;
__location(0xFF6B) u8 probe_obj_data;
__location(0xC100) u16 probe_results[4];
u16 probe_failures;
u16 probe_checks;

// Read the two bytes independently: palette reads never auto-increment.
u16 probe_read_color(u8 object, u8 slot) {
    u8 lo;
    u8 hi;
    if (object) {
        probe_obj_index=(u8)(slot*2); lo=probe_obj_data;
        probe_obj_index=(u8)(slot*2+1); hi=probe_obj_data;
    } else {
        probe_bg_index=(u8)(slot*2); lo=probe_bg_data;
        probe_bg_index=(u8)(slot*2+1); hi=probe_bg_data;
    }
    return (u16)((u16)lo | ((u16)hi<<8));
}

// Compare a full word, including a high byte different from its low byte.
void probe_expect(u8 object,u8 slot,u16 expected) {
    probe_checks++;
    if (probe_read_color(object,slot)!=expected) probe_failures++;
}

void main() {
    u8 slot;
    u16 color;
    probe_results[0]=0; probe_failures=0; probe_checks=0;
    probe_results[3]=__cgb_is_cgb();
    if (probe_results[3]) {
        __wait_vblank(); probe_lcdc=0;
        // Seed both stores so a missing high-byte write cannot pass accidentally.
        probe_bg_index=128; probe_obj_index=128;
        for(slot=0;slot<64;slot++) { probe_bg_data=0x55; probe_obj_data=0x66; }
        // Exercise every BG and OBJ slot, including the final word at bytes 62/63.
        for(slot=0;slot<32;slot++) {
            color=(u16)(0x1200+(u16)slot*3+1);
            cgb_bg_color((u8)(slot>>2),(u8)(slot&3),color);
            probe_expect(0,slot,color);
            color=(u16)(0x2300+(u16)slot*5+2);
            cgb_obj_color((u8)(slot>>2),(u8)(slot&3),color);
            probe_expect(1,slot,color);
        }
        // The component-based wrappers use the same single-color writers.
        for(slot=0;slot<32;slot++) {
            cgb_bg_rgb((u8)(slot>>2),(u8)(slot&3),slot,7,19);
            probe_expect(0,slot,CGB_RGB15(slot,7,19));
            cgb_obj_rgb((u8)(slot>>2),(u8)(slot&3),11,slot,23);
            probe_expect(1,slot,CGB_RGB15(11,slot,23));
        }
        // Palette/color inputs wrap independently; BG and OBJ memory stay separate.
        cgb_bg_color(9,7,0x1357); cgb_obj_color(9,7,0x2468);
        probe_expect(0,7,0x1357); probe_expect(1,7,0x2468);
        cgb__write_color_bg_raw(63,0x3142); cgb__write_color_obj_raw(63,0x5273);
        probe_expect(0,31,0x3142); probe_expect(1,31,0x5273);
    }
    probe_results[1]=probe_failures; probe_results[2]=probe_checks;
    probe_results[0]=0xA55A;
    while(1) {}
}

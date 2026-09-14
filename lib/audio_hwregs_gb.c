// Minimal Game Boy APU and wave RAM register declarations for audio.c.
// Compile this file before audio.c, or keep using your project's existing
// hardware-register translation unit if it already declares these symbols.

// CH1 pulse registers: sweep, duty/length, envelope and frequency/trigger.
__location(0xFF10) u8 NR10;
__location(0xFF11) u8 NR11;
__location(0xFF12) u8 NR12;
__location(0xFF13) u8 NR13;
__location(0xFF14) u8 NR14;
// CH2 pulse registers: duty/length, envelope and frequency/trigger; no sweep unit.
__location(0xFF16) u8 NR21;
__location(0xFF17) u8 NR22;
__location(0xFF18) u8 NR23;
__location(0xFF19) u8 NR24;
// CH3 wave registers: DAC enable, length, output level and frequency/trigger.
__location(0xFF1A) u8 NR30;
__location(0xFF1B) u8 NR31;
__location(0xFF1C) u8 NR32;
__location(0xFF1D) u8 NR33;
__location(0xFF1E) u8 NR34;
// CH4 noise registers: length, envelope, polynomial counter and trigger.
__location(0xFF20) u8 NR41;
__location(0xFF21) u8 NR42;
__location(0xFF22) u8 NR43;
__location(0xFF23) u8 NR44;
// Global output level, channel routing and APU power/status registers.
__location(0xFF24) u8 NR50;
__location(0xFF25) u8 NR51;
__location(0xFF26) u8 NR52;

// Sixteen wave RAM bytes hold 32 packed four-bit samples. Audio code must
// respect active-wave-channel access restrictions when replacing them.
__location(0xFF30) u8 WAVE0;
__location(0xFF31) u8 WAVE1;
__location(0xFF32) u8 WAVE2;
__location(0xFF33) u8 WAVE3;
__location(0xFF34) u8 WAVE4;
__location(0xFF35) u8 WAVE5;
__location(0xFF36) u8 WAVE6;
__location(0xFF37) u8 WAVE7;
__location(0xFF38) u8 WAVE8;
__location(0xFF39) u8 WAVE9;
__location(0xFF3A) u8 WAVE10;
__location(0xFF3B) u8 WAVE11;
__location(0xFF3C) u8 WAVE12;
__location(0xFF3D) u8 WAVE13;
__location(0xFF3E) u8 WAVE14;
__location(0xFF3F) u8 WAVE15;

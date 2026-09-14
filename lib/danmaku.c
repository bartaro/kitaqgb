#include "danmaku.h"
#pragma bank 1

// The assembly walker uses 96 records of eight bytes and a 576-byte map directly.
// Changing the public capacity/layout macros alone does not resize its loop constants.
DanmakuBullet dm_bullets[DANMAKU_MAX];
__wram __aligned(16) u8 dm_map[DANMAKU_MAP_BYTES];
u8 dm_count;
u8 dm_peak;
u8 dm_hit;
u8 dm_graze;
u8 dm_player_x;
u8 dm_player_y;
u8 dm_invulnerable;
u16 dm_spawned;
u16 dm_rejected;
static u8 dm_cursor;
u8 dm_remaining;
u8 dm_px;
u8 dm_py;
u8 dm_vy;
// Direction samples use scale 64. Fan generation multiplies by speed and shifts six bits to Q4.4 velocity.
__prg_rom s8 dm_cos[32] = {64,63,59,53,45,36,24,12,0,-12,-24,-36,-45,-53,-59,-63,-64,-63,-59,-53,-45,-36,-24,-12,0,12,24,36,45,53,59,63};
__prg_rom s8 dm_sin[32] = {0,12,24,36,45,53,59,63,64,63,59,53,45,36,24,12,0,-12,-24,-36,-45,-53,-59,-63,-64,-63,-59,-53,-45,-36,-24,-12};
__prg_rom u8 dm_bits[4] = {1,2,4,8};

// Deactivate all slots and reset active/event/cursor state; keep lifetime counters, player settings and map bytes.
void danmaku_clear() {
    u8 i;
    for (i=0; i<DANMAKU_MAX; i++) dm_bullets[i].active=0;
    dm_count=0; dm_hit=0; dm_graze=0; dm_cursor=0;
}
// Clear the pool and its peak/spawn/rejection counters. The caller still initializes the player and map.
void danmaku_reset() {
    danmaku_clear(); dm_peak=0; dm_spawned=0; dm_rejected=0;
}
// Reject off-field origins, then search from a rotating cursor for a free slot.
// Return one on admission; only a full pool increments dm_rejected, while out-of-range origins do not.
u8 danmaku_spawn(u8 x, u8 y, s8 vx, s8 vy) {
    u8 n;
    u8 i;
    if (x>=160 || y>=128) return 0;
    for(n=0;n<DANMAKU_MAX;n++) {
        i=dm_cursor;
        dm_cursor++;
        if(dm_cursor>=DANMAKU_MAX) dm_cursor=0;
        if(dm_bullets[i].active==0) {
            dm_bullets[i].x=(s16)((u16)x<<4);
            dm_bullets[i].y=(s16)((u16)y<<4);
            dm_bullets[i].vx=vx; dm_bullets[i].vy=vy;
            dm_bullets[i].active=1; dm_bullets[i].grazed=0;
            dm_count++; dm_spawned++;
            if(dm_count>dm_peak) dm_peak=dm_count;
            return 1;
        }
    }
    dm_rejected++;
    return 0;
}
// Emit up to 96 attempts in 32 wrapping directions; speed is capped at 64 sixteenth-pixels per step.
// Individual admission failures do not stop the fan. Lifetime counters retain the results.
void danmaku_fan(u8 x,u8 y,u8 direction,u8 step,u8 count,u8 speed) {
    u8 n;
    u8 a;
    s8 vx;
    s8 vy;
    if(count>DANMAKU_MAX) count=DANMAKU_MAX;
    if(speed>64) speed=64;
    a=direction&31;
    for(n=0;n<count;n++) {
        vx=(s8)(((s16)dm_cos[a]*(s16)speed)>>6);
        vy=(s8)(((s16)dm_sin[a]*(s16)speed)>>6);
        danmaku_spawn(x,y,vx,vy);
        a=(u8)((a+step)&31);
    }
}
// Clear all 576 bytes with 72 unrolled eight-byte stores. This changes the RAM map, not VRAM.
void danmaku_clear_map() {
    __asm {
        LD_HL_IMM dm_map
        LD_B_IMM 72
        XOR_A
    dm_clm_loop:
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        DEC_B
        JR_NZ dm_clm_loop
        RET
    }
}
// Advance active bullets, cull outside 160x128, compute events and OR quadrant IDs into dm_map.
// Call clear_map and restore decoration first when old bullet trails should disappear.
void danmaku_step() {
    // One linear pass. No per-bullet bank calls, multiplication or division.
    // The 8-byte packed public layout is x16,y16,vx8,vy8,active,graze.
    __asm {
        XOR_A
        LD_MEM_A dm_hit
        LD_MEM_A dm_graze
        LD_A_IMM 96
        LD_MEM_A dm_remaining
        LD_HL_IMM dm_bullets
    dm_tick_loop:
        PUSH_HL
        LD_BC_IMM 6
        ADD_HL_BC
        LD_A_HL
        OR_A
        JP_Z dm_tick_next
        DEC_HL
        LD_A_HL
        LD_MEM_A dm_vy
        DEC_HL
        LD_A_HL
        LD_C_A
        LD_B_IMM 0
        AND_IMM 128
        JR_Z dm_tick_xsign
        DEC_B
    // B:C is the sign-extended X velocity. Add to Q12.4 position; high byte >=10 also rejects negative X.
    dm_tick_xsign:
        POP_HL
        PUSH_HL
        LD_A_HL
        ADD_C
        LDI_HL_A
        LD_E_A
        LD_A_HL
        ADC_B
        LDI_HL_A
        CP_IMM 10
        JP_NC dm_tick_kill
        RRCA
        RRCA
        RRCA
        RRCA
        LD_D_A
        LD_A_E
        RRCA
        RRCA
        RRCA
        RRCA
        AND_IMM 15
        OR_D
        LD_MEM_A dm_px
        LD_A_MEM dm_vy
        LD_C_A
        LD_B_IMM 0
        AND_IMM 128
        JR_Z dm_tick_ysign
        DEC_B
    // Apply signed Y velocity similarly; high byte >=8 rejects negative Y and positions at or below 128 pixels.
    dm_tick_ysign:
        LD_A_HL
        ADD_C
        LDI_HL_A
        LD_E_A
        LD_A_HL
        ADC_B
        LDI_HL_A
        CP_IMM 8
        JP_NC dm_tick_kill
        RRCA
        RRCA
        RRCA
        RRCA
        LD_D_A
        LD_A_E
        RRCA
        RRCA
        RRCA
        RRCA
        AND_IMM 15
        OR_D
        LD_MEM_A dm_py
        // Signed differences shifted into 0..16 for a wrap-safe range test.
        // Rendered centers are (pixel & FC)+2. Test a square of radius eight around the player for graze eligibility.
        LD_A_MEM dm_player_x
        LD_C_A
        LD_A_MEM dm_px
        AND_IMM 252
        ADD_A_IMM 10
        SUB_C
        CP_IMM 17
        JR_NC dm_tick_plot
        LD_B_A
        LD_A_MEM dm_player_y
        LD_C_A
        LD_A_MEM dm_py
        AND_IMM 252
        ADD_A_IMM 10
        SUB_C
        CP_IMM 17
        JR_NC dm_tick_plot
        LD_C_A
        INC_HL
        INC_HL
        INC_HL
        LD_A_HL
        OR_A
        JR_NZ dm_tick_check_hit
        LD_A_IMM 1
        LD_HL_A
        LD_A_MEM dm_graze
        INC_A
        LD_MEM_A dm_graze
    // A bullet grazes at most once during its lifetime. Invulnerability suppresses hits, but not first-graze events.
    // The inner hit test uses a square of radius three around the same quantized center.
    dm_tick_check_hit:
        LD_A_MEM dm_invulnerable
        OR_A
        JR_NZ dm_tick_plot
        LD_A_B
        SUB_IMM 5
        CP_IMM 7
        JR_NC dm_tick_plot
        LD_A_C
        SUB_IMM 5
        CP_IMM 7
        JR_NC dm_tick_plot
        LD_A_IMM 1
        LD_MEM_A dm_hit
    // Select one of four quadrant bits from pixel bit 2, then address map[(y>>3)*32+(x>>3)].
    // Decoration IDs >=16 are replaced at occupied cells; bullet IDs 0..15 merge with OR.
    dm_tick_plot:
        LD_A_MEM dm_py
        AND_IMM 4
        AND_IMM 254
        RRCA
        LD_B_A
        LD_A_MEM dm_px
        AND_IMM 4
        AND_IMM 254
        RRCA
        AND_IMM 254
        RRCA
        OR_B
        LD_E_A
        LD_D_IMM 0
        LD_HL_IMM dm_bits
        ADD_HL_DE
        LD_A_HL
        LD_C_A
        LD_A_MEM dm_py
        AND_IMM 120
        LD_L_A
        LD_H_IMM 0
        ADD_HL_HL
        ADD_HL_HL
        LD_A_MEM dm_px
        AND_IMM 254
        RRCA
        AND_IMM 254
        RRCA
        AND_IMM 254
        RRCA
        LD_E_A
        LD_D_IMM 0
        ADD_HL_DE
        LD_DE_IMM dm_map
        ADD_HL_DE
        LD_A_HL
        CP_IMM 16
        JR_C dm_tick_merge
        XOR_A
    dm_tick_merge:
        OR_C
        LD_HL_A
        JR dm_tick_next
    // Restore the saved record base, clear active at offset six and decrement the live count exactly once.
    dm_tick_kill:
        POP_HL
        PUSH_HL
        LD_BC_IMM 6
        ADD_HL_BC
        XOR_A
        LD_HL_A
        LD_A_MEM dm_count
        DEC_A
        LD_MEM_A dm_count
    // Restore the saved base even for inactive or culled bullets, then advance by the fixed eight-byte record size.
    dm_tick_next:
        POP_HL
        LD_BC_IMM 8
        ADD_HL_BC
        LD_A_MEM dm_remaining
        DEC_A
        LD_MEM_A dm_remaining
        JP_NZ dm_tick_loop
        RET
    }
}
// Start CGB general DMA for 36 blocks (576 bytes) from aligned dm_map to 9800 in VRAM bank zero.
// This synchronous path does not detect CGB, wait for access or preserve the previous VBK value.
void danmaku_present_now() {
    __asm {
        XOR_A
        LDH_MEM_A 0x4F
        LD_HL_IMM dm_map
        LD_A_H
        LDH_MEM_A 0x51
        LD_A_L
        LDH_MEM_A 0x52
        LD_A_IMM 0x18
        LDH_MEM_A 0x53
        XOR_A
        LDH_MEM_A 0x54
        LD_A_IMM 35
        LDH_MEM_A 0x55
        RET
    }
}

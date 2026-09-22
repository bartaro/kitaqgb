// Copyright (c) 2026 DAISUKE OBA. MIT License.
// Independent bounded decoder for the ZX0 forward v2 wire format.
// Format designed by Einar Saukas; no upstream decoder code is incorporated.
#include "zx0.h"

u8 zx0_error;
static const u8* kq_zx_input;
static u16 kq_zx_left;
static u8 kq_zx_bits;
static u8 kq_zx_mask;
static u8 kq_zx_borrowed;
static u8 kq_zx_borrow_value;

// Consume one interleaved data/control byte within the supplied source extent.
static u8 kq_zx_byte() {
    u8 value;
    if(kq_zx_left==0) { zx0_error=ZX0_TRUNCATED; return 0; }
    value=*kq_zx_input; kq_zx_input++; kq_zx_left--; return value;
}
// A new-offset byte supplies one length-control bit without consuming a
// control-stream bit. Other control bits run from bit 7 down to bit 0.
static u8 kq_zx_bit() {
    u8 value;
    if(kq_zx_borrowed) { kq_zx_borrowed=0; return kq_zx_borrow_value; }
    if(kq_zx_mask==0) { kq_zx_bits=kq_zx_byte(); kq_zx_mask=128; }
    value=(u8)((kq_zx_bits&kq_zx_mask)!=0); kq_zx_mask>>=1; return value;
}
// Decode a positive 16-bit interlaced gamma integer. V2 complements only
// the payload bits of the new-offset upper part, not its continuation bits.
static u16 kq_zx_number(u8 invert) {
    u16 value;
    value=1;
    while(kq_zx_bit()==0) {
        if(zx0_error) return 0;
        if(value>=32768) { zx0_error=ZX0_BAD_STREAM; return 0; }
        value=(u16)((value<<1)|(kq_zx_bit()^invert));
    }
    return value;
}
// Reject wrapping pointers and source/destination overlap before decoding.
static u8 kq_zx_arguments(void* dst,u16 capacity,const void* src,u16 size) {
    u16 destination; u16 source;
    if(dst==0||src==0||size==0) return 0;
    destination=(u16)dst; source=(u16)src;
    if(size-1>(u16)(65535-source)) return 0;
    if(capacity!=0&&capacity-1>(u16)(65535-destination)) return 0;
    if(destination>=source) { if(destination-source<size) return 0; }
    else if(source-destination<capacity) return 0;
    return 1;
}

u16 zx0_decompress(void* dst,u16 capacity,const void* src,u16 packed_size) {
    u8 phase; u8 low; u8* out; u8* match;
    u16 count; u16 distance; u16 used; u16 upper;
    zx0_error=ZX0_OK;
    if(!kq_zx_arguments(dst,capacity,src,packed_size)) { zx0_error=ZX0_BAD_ARGUMENT; return 0; }
    kq_zx_input=(const u8*)src; kq_zx_left=packed_size;
    kq_zx_mask=0; kq_zx_borrowed=0;
    out=(u8*)dst; used=0; distance=1; phase=0;
    // Phase 0 is literal, 1 reuses the last distance, 2 carries a new distance.
    while(1) {
        if(phase==2) {
            upper=kq_zx_number(1);
            if(zx0_error) return 0;
            if(upper==256) {
                if(kq_zx_left!=0) { zx0_error=ZX0_BAD_STREAM; return 0; }
                return used;
            }
            if(upper>255) { zx0_error=ZX0_BAD_STREAM; return 0; }
            low=kq_zx_byte();
            distance=(u16)((upper<<7)-(low>>1));
            kq_zx_borrowed=1; kq_zx_borrow_value=(u8)(low&1);
            count=kq_zx_number(0);
            if(count==65535) { zx0_error=ZX0_BAD_STREAM; return 0; }
            count++;
        } else count=kq_zx_number(0);
        if(zx0_error) return 0;
        if(count>capacity-used) { zx0_error=ZX0_OUTPUT_FULL; return 0; }
        if(phase==0) {
            if(count>kq_zx_left) { zx0_error=ZX0_TRUNCATED; return 0; }
            kq_zx_left-=count; used+=count;
            while(count!=0) { *out++=*kq_zx_input++; count--; }
            phase=kq_zx_bit()?2:1;
        } else {
            if(distance>used) { zx0_error=ZX0_BAD_STREAM; return 0; }
            match=out-distance; used+=count;
            // Forward byte copying deliberately supports overlapping matches.
            while(count!=0) { *out++=*match++; count--; }
            phase=kq_zx_bit()?2:0;
        }
        if(zx0_error) return 0;
    }
}

u16 asset_decompress(void* dst,u16 capacity,const void* src,u16 packed_size) {
    const u8* in; u8* out; u16 raw; u16 packed; u16 used;
    u8 codec; u8 count; u8 value;
    zx0_error=ZX0_OK;
    if(packed_size<9||!kq_zx_arguments(dst,capacity,src,packed_size)) { zx0_error=ZX0_BAD_ARGUMENT; return 0; }
    in=(const u8*)src; out=(u8*)dst;
    if(in[0]!=75||in[1]!=81||in[2]!=65||in[3]!=49) { zx0_error=ZX0_BAD_STREAM; return 0; }
    codec=in[4]; raw=(u16)((u16)in[5]|((u16)in[6]<<8));
    packed=(u16)((u16)in[7]|((u16)in[8]<<8));
    if(packed!=packed_size-9||codec>2) { zx0_error=ZX0_BAD_STREAM; return 0; }
    if(raw>capacity) { zx0_error=ZX0_OUTPUT_FULL; return 0; }
    in+=9;
    if(codec==2) {
        used=zx0_decompress(dst,raw,in,packed);
        if(zx0_error==0&&used!=raw) zx0_error=ZX0_BAD_STREAM;
        return zx0_error?0:used;
    }
    if(codec==0) {
        if(raw!=packed) { zx0_error=ZX0_BAD_STREAM; return 0; }
        used=raw; while(raw!=0) { *out++=*in++; raw--; } return used;
    }
    used=0;
    while(packed!=0) {
        count=*in++; packed--;
        if(count==0) {
            if(packed!=0||used!=raw) { zx0_error=ZX0_BAD_STREAM; return 0; }
            return used;
        }
        if(packed==0) { zx0_error=ZX0_TRUNCATED; return 0; }
        value=*in++; packed--;
        if((u16)count>raw-used) { zx0_error=ZX0_OUTPUT_FULL; return 0; }
        used+=count; while(count!=0) { *out++=value; count--; }
    }
    zx0_error=ZX0_TRUNCATED; return 0;
}

__location(0xFF40) u8 kq_zx_lcdc;
u16 zx0_decompress_vram(u16 vram_addr,const void* src,u16 packed_size) {
    zx0_error=ZX0_OK;
    if(vram_addr<0x8000||vram_addr>=0xA000) { zx0_error=ZX0_BAD_ARGUMENT; return 0; }
    if(kq_zx_lcdc&0x80) { zx0_error=ZX0_DISPLAY_ACTIVE; return 0; }
    return zx0_decompress((void*)vram_addr,(u16)(0xA000-vram_addr),src,packed_size);
}

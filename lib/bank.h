#pragma once

// A bank byte accompanies the CPU-visible ROM address; the pointer alone is not a far address.
typedef __packed struct {
    u8 bank;
    const u8* ptr;
} BankPtr;

extern u8 kq_bank_current;

void __bankswitch(u8 bank);
u8 __farpeek8(u8 bank, const void* addr);
u16 __farpeek16(u8 bank, const void* addr);
void __farmemcpy(void* dst, u8 bank, const void* src, u16 len);
void __farcall_ptr(u8 bank, const void* func);

// Update both the library bank record and the compiler shadow used by far
// reads and calls. Invoke this permanent bank change from fixed-bank code.
void bank_switch(u8 bank);
// Return the last bank recorded by bank_switch; this does not read cartridge hardware.
u8 bank_get_current();
// Read one byte through the compiler intrinsic that accepts an explicit ROM bank.
u8 far_data_read8(u8 bank, const void* addr);
// Read a 16-bit value from the supplied bank and address through the far-read intrinsic.
u16 far_data_read16(u8 bank, const void* addr);
// Copy len bytes from banked ROM into caller-owned writable storage.
void far_data_read(u8 bank, const void* addr, void* dst, u16 len);
// Invoke a no-argument callback in its ROM bank, then restore the caller
// bank. The pointer must identify executable code; no arguments are forwarded.
void far_call(u8 bank, const void* func);
// Store a bank/address pair without accessing ROM; a null output pointer is ignored.
void farptr_make(BankPtr* out, u8 bank, const u8* ptr);
// Read one byte using both components of the bank-qualified pointer.
u8 farptr_read8(BankPtr ptr);
// Read a 16-bit value using both components of the bank-qualified pointer.
u16 farptr_read16(BankPtr ptr);
// Copy len bytes from a bank-qualified pointer; dst must have room for the full copy.
void farptr_read(BankPtr ptr, void* dst, u16 len);

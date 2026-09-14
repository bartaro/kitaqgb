#include "bank.h"

// Software shadow starts at zero; initialize it through bank_switch before relying on it.
u8 kq_bank_current;

// Update both the library bank record and the compiler shadow used by far
// reads and calls. Invoke this permanent bank change from fixed-bank code.
void bank_switch(u8 bank)
{
    kq_bank_current = bank;
    __bankswitch(bank);
}

// Return the last bank recorded by bank_switch; this does not read cartridge hardware.
u8 bank_get_current()
{
    return kq_bank_current;
}

// Read one byte through the compiler intrinsic that accepts an explicit ROM bank.
u8 far_data_read8(u8 bank, const void* addr)
{
    return __farpeek8(bank, addr);
}

// Read a 16-bit value from the supplied bank and address through the far-read intrinsic.
u16 far_data_read16(u8 bank, const void* addr)
{
    return __farpeek16(bank, addr);
}

// Copy len bytes from banked ROM into caller-owned writable storage.
void far_data_read(u8 bank, const void* addr, void* dst, u16 len)
{
    __farmemcpy(dst, bank, addr, len);
}

// Invoke a no-argument callback in its ROM bank, then restore the caller
// bank. The pointer must identify executable code; no arguments are forwarded.
void far_call(u8 bank, const void* func)
{
    __farcall_ptr(bank, func);
}

// Store a bank/address pair without accessing ROM; a null output pointer is ignored.
void farptr_make(BankPtr* out, u8 bank, const u8* ptr)
{
    if (out == 0) return;
    out->bank = bank;
    out->ptr = ptr;
}

// Read one byte using both components of the bank-qualified pointer.
u8 farptr_read8(BankPtr ptr)
{
    return __farpeek8(ptr.bank, ptr.ptr);
}

// Read a 16-bit value using both components of the bank-qualified pointer.
u16 farptr_read16(BankPtr ptr)
{
    return __farpeek16(ptr.bank, ptr.ptr);
}

// Copy len bytes from a bank-qualified pointer; dst must have room for the full copy.
void farptr_read(BankPtr ptr, void* dst, u16 len)
{
    __farmemcpy(dst, ptr.bank, ptr.ptr, len);
}

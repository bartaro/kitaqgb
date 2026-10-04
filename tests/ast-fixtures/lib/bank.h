#pragma once

typedef __packed struct {
    u8 bank;
    const u8* ptr;
} BankPtr;

extern u8 kq_bank_current;

u8 __farpeek8(u8 bank, const void* addr);
u16 __farpeek16(u8 bank, const void* addr);
void __farmemcpy(void* dst, u8 bank, const void* src, u16 len);
void __farcall_ptr(u8 bank, const void* func);

void bank_switch(u8 bank);
u8 bank_get_current();
u8 far_data_read8(u8 bank, const void* addr);
u16 far_data_read16(u8 bank, const void* addr);
void far_data_read(u8 bank, const void* addr, void* dst, u16 len);
void far_call(u8 bank, const void* func);
void farptr_make(BankPtr* out, u8 bank, const u8* ptr);
u8 farptr_read8(BankPtr ptr);
u16 farptr_read16(BankPtr ptr);
void farptr_read(BankPtr ptr, void* dst, u16 len);

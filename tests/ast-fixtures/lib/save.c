#include "rpg.h"

#define SAVE_HEADER_SIZE ((u16)6)

static void save_enable()
{
    *(u8*)0x4000 = 0;
    *(u8*)0x0000 = 0x0A;
}

static void save_disable()
{
    *(u8*)0x0000 = 0x00;
}

static void save_select_bank(u8 bank)
{
    *(u8*)0x4000 = bank;
}

static u8 save_read_abs(u16 abs_off)
{
    u8 bank = (u8)(abs_off >> 13);
    u16 addr = (u16)(0xA000 + (abs_off & 0x1FFF));
    save_select_bank(bank);
    return *(u8*)addr;
}

static void save_write_abs(u16 abs_off, u8 value)
{
    u8 bank = (u8)(abs_off >> 13);
    u16 addr = (u16)(0xA000 + (abs_off & 0x1FFF));
    save_select_bank(bank);
    *(u8*)addr = value;
}

static u8 save_checksum_ram(u16 abs_off, u16 len)
{
    u8 sum = 0;
    while (len != 0) {
        sum = (u8)(sum + save_read_abs(abs_off));
        abs_off++;
        len--;
    }
    return sum;
}

static u16 save_slot_base(u8 slot)
{
    return (u16)((u16)slot * SAVE_SLOT_SIZE);
}

void save_init()
{
    save_enable();
    save_disable();
}

u8 save_write(u8 slot, const void* data, u16 len)
{
    const u8* src = (const u8*)data;
    u16 base;
    u16 i;
    u8 sum = 0;

    if (slot >= SAVE_SLOT_COUNT) return 0;
    if (len > (u16)(SAVE_SLOT_SIZE - SAVE_HEADER_SIZE)) return 0;

    base = save_slot_base(slot);
    save_enable();

    i = 0;
    while (i < len) {
        u8 v = src[i];
        save_write_abs((u16)(base + SAVE_HEADER_SIZE + i), v);
        sum = (u8)(sum + v);
        i++;
    }

    save_write_abs(base + 0, SAVE_MAGIC0);
    save_write_abs(base + 1, SAVE_MAGIC1);
    save_write_abs(base + 2, SAVE_VERSION);
    save_write_abs(base + 3, (u8)(len & 0xFF));
    save_write_abs(base + 4, (u8)(len >> 8));
    save_write_abs(base + 5, sum);

    save_disable();
    return 1;
}

u8 save_load(u8 slot, void* data, u16 len)
{
    return save_read(slot, data, len);
}

u8 save_read(u8 slot, void* data, u16 len)
{
    u8* dst = (u8*)data;
    u16 base;
    u16 stored_len;
    u8 stored_sum;
    u16 i;

    if (slot >= SAVE_SLOT_COUNT) return 0;

    base = save_slot_base(slot);
    save_enable();

    if (save_read_abs(base + 0) != SAVE_MAGIC0) {
        save_disable();
        return 0;
    }
    if (save_read_abs(base + 1) != SAVE_MAGIC1) {
        save_disable();
        return 0;
    }
    if (save_read_abs(base + 2) != SAVE_VERSION) {
        save_disable();
        return 0;
    }

    stored_len = (u16)save_read_abs(base + 3);
    stored_len |= (u16)((u16)save_read_abs(base + 4) << 8);
    stored_sum = save_read_abs(base + 5);

    if (stored_len != len || stored_len > (u16)(SAVE_SLOT_SIZE - SAVE_HEADER_SIZE)) {
        save_disable();
        return 0;
    }
    if (save_checksum_ram((u16)(base + SAVE_HEADER_SIZE), stored_len) != stored_sum) {
        save_disable();
        return 0;
    }

    i = 0;
    while (i < stored_len) {
        dst[i] = save_read_abs((u16)(base + SAVE_HEADER_SIZE + i));
        i++;
    }

    save_disable();
    return 1;
}

u8 save_check(u8 slot)
{
    return save_exists(slot);
}

u8 save_exists(u8 slot)
{
    u16 base;
    u16 stored_len;
    u8 stored_sum;

    if (slot >= SAVE_SLOT_COUNT) return 0;

    base = save_slot_base(slot);
    save_enable();

    if (save_read_abs(base + 0) != SAVE_MAGIC0) {
        save_disable();
        return 0;
    }
    if (save_read_abs(base + 1) != SAVE_MAGIC1) {
        save_disable();
        return 0;
    }
    if (save_read_abs(base + 2) != SAVE_VERSION) {
        save_disable();
        return 0;
    }

    stored_len = (u16)save_read_abs(base + 3);
    stored_len |= (u16)((u16)save_read_abs(base + 4) << 8);
    stored_sum = save_read_abs(base + 5);

    if (stored_len > (u16)(SAVE_SLOT_SIZE - SAVE_HEADER_SIZE)) {
        save_disable();
        return 0;
    }
    if (save_checksum_ram((u16)(base + SAVE_HEADER_SIZE), stored_len) != stored_sum) {
        save_disable();
        return 0;
    }

    save_disable();
    return 1;
}

void save_clear(u8 slot)
{
    u16 base;
    u16 i;

    if (slot >= SAVE_SLOT_COUNT) return;

    base = save_slot_base(slot);
    save_enable();

    i = 0;
    while (i < SAVE_SLOT_SIZE) {
        save_write_abs((u16)(base + i), 0);
        i++;
    }

    save_disable();
}

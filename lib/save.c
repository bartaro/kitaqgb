#include "rpg.h"

// Each slot starts with two magic bytes, a version, a little-endian payload
// length and an 8-bit additive checksum. This format detects some corruption
// but is neither authenticated nor an atomic power-loss-safe transaction.
// The configured slot size must be at least six bytes; all slot extents must fit available RAM and u16 offsets.
#define SAVE_HEADER_SIZE ((u16)6)

// Select RAM bank zero and enable cartridge RAM through the expected mapper
// registers. The cartridge must support this register layout.
// Mapper banking mode, physical RAM capacity and interrupt coordination are caller responsibilities.
static void save_enable()
{
    *(u8*)0x4000 = 0;
    *(u8*)0x0000 = 0x0A;
}

// Disable cartridge RAM after an operation; the prior RAM-bank state is not restored.
static void save_disable()
{
    *(u8*)0x0000 = 0x00;
}

// Select the cartridge RAM bank required for the next absolute-offset access.
static void save_select_bank(u8 bank)
{
    *(u8*)0x4000 = bank;
}

// Split a 16-bit absolute save offset into an 8 KiB RAM bank and CPU address.
// RAM must already be enabled, and the requested physical bank must exist.
static u8 save_read_abs(u16 abs_off)
{
    u8 bank = (u8)(abs_off >> 13);
    u16 addr = (u16)(0xA000 + (abs_off & 0x1FFF));
    save_select_bank(bank);
    return *(u8*)addr;
}

// Select the 8 KiB bank containing the offset and write through its CPU window.
static void save_write_abs(u16 abs_off, u8 value)
{
    u8 bank = (u8)(abs_off >> 13);
    u16 addr = (u16)(0xA000 + (abs_off & 0x1FFF));
    save_select_bank(bank);
    *(u8*)addr = value;
}

// Sum the stored payload modulo 256 while allowing reads to cross RAM-bank boundaries.
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

// Compute the slot's absolute byte offset. Configured slot sizes/counts must
// fit the 16-bit offset range and the cartridge's available RAM.
static u16 save_slot_base(u8 slot)
{
    return (u16)((u16)slot * SAVE_SLOT_SIZE);
}

// Initialize the expected RAM access state and leave cartridge RAM disabled.
// Existing slot contents are preserved.
void save_init()
{
    save_enable();
    save_disable();
}

// Validate slot and payload length, write payload bytes, then write the header
// and checksum. Return one after completing the writes; interruption can leave
// a partially replaced slot, so this is not transactional save storage.
// Supply len readable source bytes outside the RAM window being switched. Success reports completed
// software writes; it does not read back hardware or verify battery-backed persistence.
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

// Alias the checked save_read operation.
u8 save_load(u8 slot, void* data, u16 len)
{
    return save_read(slot, data, len);
}

// Require matching magic/version, exact requested length and a valid checksum
// before copying data into the destination. Validation failure leaves the
// destination unchanged. An invalid slot returns before touching RAM control;
// validation failures after enabling RAM disable it before returning zero.
// Supply len writable destination bytes outside the banked cartridge-RAM window.
// Do not allow concurrent writers between checksum validation and the subsequent copy.
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

// Alias save_exists, including its header and checksum validation.
u8 save_check(u8 slot)
{
    return save_exists(slot);
}

// Validate a slot's header, payload bounds and checksum without copying the
// payload out. A nonzero result means the stored record passes these checks.
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

// Zero the complete selected slot, including header and payload; invalid slot
// IDs are ignored. Cartridge RAM is disabled when clearing finishes.
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

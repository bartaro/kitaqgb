#include "rpg.h"

static u8 text_x;
static u8 text_y;
static u8 text_w;
static u8 text_h;
static u8 text_cx;
static u8 text_cy;
static u8 text_speed;
static u8 text_opened;

// Block for the requested number of VBlank waits; this does not advance other game services.
static void text_delay_frames(u8 frames)
{
    while (frames != 0) {
        __wait_vblank();
        frames--;
    }
}

// Exclude the left and right borders, clamping a too-narrow window to zero usable columns.
static u8 text_inner_w()
{
    if (text_w <= 2) return 0;
    return (u8)(text_w - 2);
}

// Exclude the top and bottom borders, clamping a too-short window to zero usable rows.
static u8 text_inner_h()
{
    if (text_h <= 2) return 0;
    return (u8)(text_h - 2);
}

// Move the text cursor to the first cell inside the current window.
static void text_reset_cursor()
{
    text_cx = 0;
    text_cy = 0;
}

// Fill only the interior cells, preserving the border and leaving the cursor unchanged.
static void text_clear_inside()
{
    u8 inner_w = text_inner_w();
    u8 inner_h = text_inner_h();
    if (inner_w == 0 || inner_h == 0) return;
    __settile_rect((u8)(text_x + 1), (u8)(text_y + 1), inner_w, inner_h, TEXT_TILE_FILL);
}

// Clear the rectangle and draw its border using the configured frame tiles.
// The caller supplies screen coordinates and dimensions that fit the tile map.
static void text_draw_frame()
{
    u8 i;

    __settile_rect(text_x, text_y, text_w, text_h, TEXT_TILE_FILL);
    if (text_w == 0 || text_h == 0) return;

    __settile_xy(text_x, text_y, TEXT_TILE_FRAME_TL);
    if (text_w > 1) __settile_xy((u8)(text_x + text_w - 1), text_y, TEXT_TILE_FRAME_TR);
    if (text_h > 1) __settile_xy(text_x, (u8)(text_y + text_h - 1), TEXT_TILE_FRAME_BL);
    if (text_w > 1 && text_h > 1) {
        __settile_xy((u8)(text_x + text_w - 1), (u8)(text_y + text_h - 1), TEXT_TILE_FRAME_BR);
    }

    i = 1;
    while (i + 1 < text_w) {
        __settile_xy((u8)(text_x + i), text_y, TEXT_TILE_FRAME_H);
        if (text_h > 1) {
            __settile_xy((u8)(text_x + i), (u8)(text_y + text_h - 1), TEXT_TILE_FRAME_H);
        }
        i++;
    }

    i = 1;
    while (i + 1 < text_h) {
        __settile_xy(text_x, (u8)(text_y + i), TEXT_TILE_FRAME_V);
        if (text_w > 1) {
            __settile_xy((u8)(text_x + text_w - 1), (u8)(text_y + i), TEXT_TILE_FRAME_V);
        }
        i++;
    }
}

// Wait for A, B or START using a local previous-button state initialized to zero.
// A button already held on entry can therefore advance the page on the first poll.
static void text_wait_advance()
{
    u8 prev = 0;
    while (1) {
        u16 kt;
        u8 trigger;
        __wait_vblank();
        kt = __readpadex(prev);
        prev = (u8)kt;
        trigger = (u8)(kt >> 8);
        if ((trigger & (PAD_KEY_A | PAD_KEY_B | PAD_KEY_START)) != 0) return;
    }
}

// Advance one interior row. At the bottom, wait for input, clear the page and
// restart at the first cell; this is pagination, not upward scrolling.
static void text_newline()
{
    text_cx = 0;
    text_cy++;
    if (text_cy >= text_inner_h()) {
        if (text_inner_w() != 0 && text_inner_h() != 0) {
            __settile_xy((u8)(text_x + text_inner_w()), (u8)(text_y + text_inner_h()), TEXT_TILE_PAGE_WAIT);
        }
        text_wait_advance();
        text_clear_inside();
        text_reset_cursor();
    }
}

// Write one tile only when a usable window is open. Wrap through the blocking
// page-advance path as needed, then apply the configured per-character delay.
static void text_put_tile(u8 tile)
{
    if (text_opened == 0) return;
    if (text_inner_w() == 0 || text_inner_h() == 0) return;
    if (text_cx >= text_inner_w()) {
        text_newline();
    }
    __settile_xy((u8)(text_x + 1 + text_cx), (u8)(text_y + 1 + text_cy), tile);
    text_cx++;
    if (text_speed != 0) {
        text_delay_frames(text_speed);
    }
}

// Draw zero-terminated tile bytes directly on one row without control decoding,
// window clipping or per-character delays; keep the source bank accessible.
static void text_draw_string_at(u8 x, u8 y, const u8* str)
{
    u8 tx = x;
    while (*str != 0) {
        __settile_xy(tx, y, *str);
        tx++;
        str++;
    }
}

// Count decimal digits, treating zero as a one-digit value.
static u8 text_dec_digits_u16(u16 value)
{
    u8 digits = 1;
    while (value >= 10) {
        value = (u16)(value / 10);
        digits++;
    }
    return digits;
}

// Clear the field and right-align decimal digits. A narrow field keeps the
// least significant digits; the caller ensures the field fits the tile map.
static void text_print_u16_width(u8 x, u8 y, u16 value, u8 width)
{
    u8 digits[5];
    u8 count = 0;
    u8 i;

    do {
        digits[4 - count] = (u8)(TEXT_DIGIT_BASE + (value % 10));
        value = (u16)(value / 10);
        count++;
    } while (value != 0 && count < 5);

    i = 0;
    while (i < width) {
        __settile_xy((u8)(x + i), y, TEXT_TILE_FILL);
        i++;
    }

    if (count > width) count = width;
    i = 0;
    while (i < count) {
        __settile_xy((u8)(x + width - count + i), y, digits[5 - count + i]);
        i++;
    }
}

// Set the tile-space window, reset its cursor and redraw it immediately.
// Opening does not reset the existing character speed or load font tiles.
void text_open(u8 x, u8 y, u8 w, u8 h)
{
    text_x = x;
    text_y = y;
    text_w = w;
    text_h = h;
    text_opened = 1;
    text_reset_cursor();
    text_draw_frame();
    text_clear_inside();
}

// Erase the open window with the fill tile and disable windowed character output.
void text_close()
{
    if (text_opened == 0) return;
    __settile_rect(text_x, text_y, text_w, text_h, TEXT_TILE_FILL);
    text_opened = 0;
}

// Set VBlank waits per windowed character; zero prints without character delays.
void text_set_speed(u8 speed)
{
    text_speed = speed;
}

// Open and redraw a window using the same tile-space arguments as text_open.
void text_window(u8 x, u8 y, u8 w, u8 h)
{
    text_open(x, y, w, h);
}

// Fill the supplied tile rectangle without changing the active text window or cursor.
void text_clear_rect(u8 x, u8 y, u8 w, u8 h)
{
    __settile_rect(x, y, w, h, TEXT_TILE_FILL);
}

// Draw a zero-terminated tile string at absolute tile coordinates, without
// window clipping, control codes or the windowed printing delay.
void text_print_xy(u8 x, u8 y, const u8* str)
{
    text_draw_string_at(x, y, str);
}

// Print an unsigned byte in decimal using only the required number of cells.
void text_print_u8(u8 x, u8 y, u8 value)
{
    text_print_u16_width(x, y, (u16)value, text_dec_digits_u16((u16)value));
}

// Print an unsigned word in decimal using only the required number of cells.
void text_print_u16(u8 x, u8 y, u16 value)
{
    text_print_u16_width(x, y, value, text_dec_digits_u16(value));
}

// Print a minus tile for negative values, followed by the unsigned magnitude.
// Positive values use the unsigned decimal path; no plus sign is emitted.
void text_print_s16(u8 x, u8 y, s16 value)
{
    u16 magnitude;
    if (value < 0) {
        __settile_xy(x, y, (u8)'-');
        magnitude = (u16)((s16)0 - value);
        text_print_u16_width((u8)(x + 1), y, magnitude, text_dec_digits_u16(magnitude));
        return;
    }
    text_print_u16(x, y, (u16)value);
}

// Decode an END-terminated stream of tile bytes and newline/wait/page controls.
// WAIT consumes a following frame-count byte. The source must remain readable
// through blocking waits; the routine does not validate stream length.
void text_print(const u8* str)
{
    while (*str != TEXT_CTRL_END) {
        u8 c = *str++;
        if (c == TEXT_CTRL_NEWLINE) {
            text_newline();
            continue;
        }
        if (c == TEXT_CTRL_WAIT) {
            u8 frames = *str++;
            text_delay_frames(frames);
            continue;
        }
        if (c == TEXT_CTRL_PAGE_WAIT) {
            text_wait_advance();
            text_clear_inside();
            text_reset_cursor();
            continue;
        }
        text_put_tile(c);
    }
}

// Decode the same text controls while fetching each byte from the specified
// ROM bank. The pointer advances within that bank; it does not roll into the next bank.
void text_print_far(u8 bank, const u8* str)
{
    const u8* p = str;
    while (1) {
        u8 c = __farpeek8(bank, p);
        p++;
        if (c == TEXT_CTRL_END) return;
        if (c == TEXT_CTRL_NEWLINE) {
            text_newline();
            continue;
        }
        if (c == TEXT_CTRL_WAIT) {
            u8 frames = __farpeek8(bank, p);
            p++;
            text_delay_frames(frames);
            continue;
        }
        if (c == TEXT_CTRL_PAGE_WAIT) {
            text_wait_advance();
            text_clear_inside();
            text_reset_cursor();
            continue;
        }
        text_put_tile(c);
    }
}

// Redraw choices and block for edge-triggered navigation: UP/DOWN wrap, A
// accepts, and B returns 0xFF. Zero choices also returns 0xFF. Choice strings
// use raw tile bytes, and the caller must fit them inside the available rows.
u8 text_choice(const u8* const* choices, u8 count)
{
    u8 cursor;
    u8 prev;

    if (count == 0) return 0xFF;

    cursor = 0;
    prev = 0;
    while (1) {
        u8 i;
        for (i = 0; i < count; i++) {
            u8 py = (u8)(text_y + 1 + i);
            __settile_xy((u8)(text_x + 1), py, (i == cursor) ? TEXT_TILE_CURSOR : TEXT_TILE_FILL);
            text_draw_string_at((u8)(text_x + 2), py, choices[i]);
        }

        __wait_vblank();
        {
            u16 kt = __readpadex(prev);
            u8 trigger;
            prev = (u8)kt;
            trigger = (u8)(kt >> 8);

            if ((trigger & PAD_KEY_UP) != 0) {
                if (cursor == 0) cursor = (u8)(count - 1);
                else cursor--;
            }
            if ((trigger & PAD_KEY_DOWN) != 0) {
                cursor++;
                if (cursor >= count) cursor = 0;
            }
            if ((trigger & PAD_KEY_A) != 0) return cursor;
            if ((trigger & PAD_KEY_B) != 0) return 0xFF;
        }
    }
}

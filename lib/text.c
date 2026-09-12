#include "rpg.h"

static u8 text_x;
static u8 text_y;
static u8 text_w;
static u8 text_h;
static u8 text_cx;
static u8 text_cy;
static u8 text_speed;
static u8 text_opened;

static void text_delay_frames(u8 frames)
{
    while (frames != 0) {
        __wait_vblank();
        frames--;
    }
}

static u8 text_inner_w()
{
    if (text_w <= 2) return 0;
    return (u8)(text_w - 2);
}

static u8 text_inner_h()
{
    if (text_h <= 2) return 0;
    return (u8)(text_h - 2);
}

static void text_reset_cursor()
{
    text_cx = 0;
    text_cy = 0;
}

static void text_clear_inside()
{
    u8 inner_w = text_inner_w();
    u8 inner_h = text_inner_h();
    if (inner_w == 0 || inner_h == 0) return;
    __settile_rect((u8)(text_x + 1), (u8)(text_y + 1), inner_w, inner_h, TEXT_TILE_FILL);
}

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

static void text_draw_string_at(u8 x, u8 y, const u8* str)
{
    u8 tx = x;
    while (*str != 0) {
        __settile_xy(tx, y, *str);
        tx++;
        str++;
    }
}

static u8 text_dec_digits_u16(u16 value)
{
    u8 digits = 1;
    while (value >= 10) {
        value = (u16)(value / 10);
        digits++;
    }
    return digits;
}

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

void text_close()
{
    if (text_opened == 0) return;
    __settile_rect(text_x, text_y, text_w, text_h, TEXT_TILE_FILL);
    text_opened = 0;
}

void text_set_speed(u8 speed)
{
    text_speed = speed;
}

void text_window(u8 x, u8 y, u8 w, u8 h)
{
    text_open(x, y, w, h);
}

void text_clear_rect(u8 x, u8 y, u8 w, u8 h)
{
    __settile_rect(x, y, w, h, TEXT_TILE_FILL);
}

void text_print_xy(u8 x, u8 y, const u8* str)
{
    text_draw_string_at(x, y, str);
}

void text_print_u8(u8 x, u8 y, u8 value)
{
    text_print_u16_width(x, y, (u16)value, text_dec_digits_u16((u16)value));
}

void text_print_u16(u8 x, u8 y, u16 value)
{
    text_print_u16_width(x, y, value, text_dec_digits_u16(value));
}

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

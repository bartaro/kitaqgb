#include "input.h"

u8 kq_input_prev_keys;
u8 kq_input_keys;
u8 kq_input_press_keys;
u8 kq_input_release_keys;
u8 kq_input_repeat_keys;
static u8 input_repeat_timer[8];

static u8 input_mask_for_index(u8 index)
{
    return (u8)(1 << index);
}

void input_init()
{
    u8 i;
    kq_input_prev_keys = 0;
    kq_input_keys = 0;
    kq_input_press_keys = 0;
    kq_input_release_keys = 0;
    kq_input_repeat_keys = 0;
    i = 0;
    while (i < 8) {
        input_repeat_timer[i] = INPUT_REPEAT_DELAY;
        i++;
    }
}

void input_update()
{
    u8 i;
    u16 kt;

    kq_input_prev_keys = kq_input_keys;
    kt = __readpadex(kq_input_prev_keys);
    kq_input_keys = (u8)kt;
    kq_input_press_keys = (u8)(kt >> 8);
    kq_input_release_keys = (u8)(kq_input_prev_keys & (u8)(kq_input_keys ^ 0xFF));
    kq_input_repeat_keys = kq_input_press_keys;

    i = 0;
    while (i < 8) {
        u8 mask = input_mask_for_index(i);
        if ((kq_input_keys & mask) != 0) {
            if ((kq_input_press_keys & mask) != 0) {
                input_repeat_timer[i] = INPUT_REPEAT_DELAY;
            } else if (input_repeat_timer[i] != 0) {
                input_repeat_timer[i]--;
            } else {
                kq_input_repeat_keys = (u8)(kq_input_repeat_keys | mask);
                input_repeat_timer[i] = INPUT_REPEAT_RATE;
            }
        } else {
            input_repeat_timer[i] = INPUT_REPEAT_DELAY;
        }
        i++;
    }
}

u8 input_down(u8 mask)
{
    return (u8)((kq_input_keys & mask) != 0);
}

u8 input_pressed(u8 mask)
{
    return (u8)((kq_input_press_keys & mask) != 0);
}

u8 input_released(u8 mask)
{
    return (u8)((kq_input_release_keys & mask) != 0);
}

u8 input_repeat(u8 mask)
{
    return (u8)((kq_input_repeat_keys & mask) != 0);
}

u8 input_current()
{
    return kq_input_keys;
}

u8 input_previous()
{
    return kq_input_prev_keys;
}

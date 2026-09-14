#include "input.h"

u8 kq_input_prev_keys;
u8 kq_input_keys;
u8 kq_input_press_keys;
u8 kq_input_release_keys;
u8 kq_input_repeat_keys;
// Each button counts down independently; getters only observe the most recent update.
static u8 input_repeat_timer[8];

// Map one of the eight button indices to its bit in the input masks.
static u8 input_mask_for_index(u8 index)
{
    return (u8)(1 << index);
}

// Clear sampled/edge state and initialize an independent repeat timer for each button.
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

// Sample input once per game tick and derive press, release and repeat masks.
// Repeat delays are measured in calls to this function, not elapsed wall-clock time.
void input_update()
{
    u8 i;
    u16 kt;

    kq_input_prev_keys = kq_input_keys;
    // The intrinsic packs held buttons in the low byte and new presses in the high byte.
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

// Return whether any masked button is currently held.
u8 input_down(u8 mask)
{
    return (u8)((kq_input_keys & mask) != 0);
}

// Return whether any masked button became pressed in the latest sample.
u8 input_pressed(u8 mask)
{
    return (u8)((kq_input_press_keys & mask) != 0);
}

// Return whether any masked button became released in the latest sample.
u8 input_released(u8 mask)
{
    return (u8)((kq_input_release_keys & mask) != 0);
}

// Return whether any masked button produced a fresh press or scheduled repeat pulse.
u8 input_repeat(u8 mask)
{
    return (u8)((kq_input_repeat_keys & mask) != 0);
}

// Return the complete held-button mask from the latest update.
u8 input_current()
{
    return kq_input_keys;
}

// Return the held-button mask that preceded the latest update.
u8 input_previous()
{
    return kq_input_prev_keys;
}

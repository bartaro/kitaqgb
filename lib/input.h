#pragma once

#define BTN_RIGHT  ((u8)0x01)
#define BTN_LEFT   ((u8)0x02)
#define BTN_UP     ((u8)0x04)
#define BTN_DOWN   ((u8)0x08)
#define BTN_A      ((u8)0x10)
#define BTN_B      ((u8)0x20)
#define BTN_SELECT ((u8)0x40)
#define BTN_START  ((u8)0x80)

// A fresh press pulses immediately. The next pulse occurs DELAY + 1 updates later.
#ifndef INPUT_REPEAT_DELAY
#define INPUT_REPEAT_DELAY ((u8)18)
#endif

// Subsequent pulses are RATE + 1 updates apart; zero repeats on every held update.
#ifndef INPUT_REPEAT_RATE
#define INPUT_REPEAT_RATE ((u8)5)
#endif

extern u8 kq_input_prev_keys;
extern u8 kq_input_keys;
extern u8 kq_input_press_keys;
extern u8 kq_input_release_keys;
extern u8 kq_input_repeat_keys;

u16 __readpadex(u8 prev_keys);

// Clear sampled/edge state and initialize an independent repeat timer for each button.
void input_init();
// Sample input once per game tick and derive press, release and repeat masks.
// Repeat delays are measured in calls to this function, not elapsed wall-clock time.
void input_update();
// Return whether any masked button is currently held.
u8 input_down(u8 mask);
// Return whether any masked button became pressed in the latest sample.
u8 input_pressed(u8 mask);
// Return whether any masked button became released in the latest sample.
u8 input_released(u8 mask);
// Return whether any masked button produced a fresh press or scheduled repeat pulse.
u8 input_repeat(u8 mask);
// Return the complete held-button mask from the latest update.
u8 input_current();
// Return the held-button mask that preceded the latest update.
u8 input_previous();

#define btn_down input_down
#define btn_pressed input_pressed
#define btn_released input_released
#define btn_repeat input_repeat

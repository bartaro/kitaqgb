#include "raster.h"

void __wait_ly(u8 ly);

static u8 Raster_CountValue;
static u8 Raster_LastLyValue;
static u8 Raster_HasLastLy;
static u8 Raster_LastErrorValue;

static __prg_rom u8 Raster_LineXXTitle[16] = {
    252, 252, 253, 253, 254, 254, 255, 255,
    0, 0, 1, 1, 2, 2, 3, 3
};

static __prg_rom u8 Raster_LineXTravelGate[64] = {
    0, 2, 5, 7, 9, 11, 13, 15,
    17, 19, 20, 21, 22, 23, 24, 24,
    24, 24, 24, 23, 22, 21, 20, 19,
    17, 15, 13, 11, 9, 7, 5, 2,
    0, 254, 251, 249, 247, 245, 243, 241,
    239, 237, 236, 235, 234, 233, 232, 232,
    232, 232, 232, 233, 234, 235, 236, 237,
    239, 241, 243, 245, 247, 249, 251, 254
};

static u8 Raster_SetError(u8 error)
{
    Raster_LastErrorValue = error;
    return error;
}

static u8 Raster_CheckPush(u8 ly)
{
    if (ly >= 144) {
        return Raster_SetError(KQ_RASTER_BAD_LY);
    }

    if (Raster_CountValue >= KQ_RASTER_MAX_BANDS) {
        return Raster_SetError(KQ_RASTER_FULL);
    }

    if (Raster_HasLastLy != 0) {
        if (ly <= Raster_LastLyValue) {
            return Raster_SetError(KQ_RASTER_UNSORTED);
        }
    }

    return KQ_RASTER_OK;
}

void Raster_Init()
{
    Raster_Clear();
}

void Raster_Clear()
{
    Scroll_SplitReset();
    Raster_CountValue = 0;
    Raster_LastLyValue = 0;
    Raster_HasLastLy = 0;
    Raster_LastErrorValue = KQ_RASTER_OK;
}

void Raster_Disable()
{
    Raster_Clear();
    Scroll_SplitCommit();
}

u8 Raster_GetCount()
{
    return Raster_CountValue;
}

u8 Raster_GetLastError()
{
    return Raster_LastErrorValue;
}

u8 Raster_Push(u8 ly, u8 scx, u8 scy)
{
    u8 r;

    r = Raster_CheckPush(ly);
    if (r != KQ_RASTER_OK) return r;

    Scroll_SplitPush(ly, scx, scy);

    Raster_LastLyValue = ly;
    Raster_HasLastLy = 1;
    Raster_CountValue++;
    Raster_LastErrorValue = KQ_RASTER_OK;
    return KQ_RASTER_OK;
}

u8 Raster_PushEx(u8 ly, u8 scx, u8 scy, u8 wx, u8 wy, u8 flags)
{
    u8 r;

    r = Raster_CheckPush(ly);
    if (r != KQ_RASTER_OK) return r;

    flags = (u8)(flags & KQ_RASTER_FLAG_MASK);
    Scroll_SplitPushEx(ly, scx, scy, wx, wy, flags);

    Raster_LastLyValue = ly;
    Raster_HasLastLy = 1;
    Raster_CountValue++;
    Raster_LastErrorValue = KQ_RASTER_OK;
    return KQ_RASTER_OK;
}

u8 Raster_PushWindowRaw(u8 ly, u8 wx, u8 wy, u8 flags)
{
    flags = (u8)(flags | KQ_RASTER_WIN);
    return Raster_PushEx(ly, 0, 0, wx, wy, flags);
}

u8 Raster_PushWindowScreen(u8 ly, u8 screen_x, u8 screen_y, u8 flags)
{
    return Raster_PushWindowRaw(ly, (u8)(screen_x + 7), screen_y, flags);
}

u8 Raster_PushBgWindowScreen(u8 ly, u8 scx, u8 scy, u8 screen_x, u8 screen_y, u8 flags)
{
    flags = (u8)(flags | KQ_RASTER_BG | KQ_RASTER_WIN);
    return Raster_PushEx(ly, scx, scy, (u8)(screen_x + 7), screen_y, flags);
}

u8 Raster_PushBgColor0(u8 ly, u16 rgb15)
{
    return Raster_PushEx(ly, (u8)rgb15, (u8)(rgb15 >> 8), 0, 0, KQ_RASTER_BG_COLOR0);
}

u8 Raster_Commit()
{
    Scroll_SplitCommit();
    return Raster_LastErrorValue;
}

u8 Raster_BuildHudTop(u8 hud_height, u8 camera_x, u8 camera_y)
{
    u8 r;

    if (hud_height == 0) return Raster_SetError(KQ_RASTER_BAD_RANGE);
    if (hud_height >= 144) return Raster_SetError(KQ_RASTER_BAD_RANGE);

    Raster_Clear();
    r = Raster_Push(0, 0, 0);
    if (r != KQ_RASTER_OK) return r;
    return Raster_Push(hud_height, camera_x, camera_y);
}

u8 Raster_BuildHudBottom(u8 hud_y, u8 camera_x, u8 camera_y)
{
    u8 r;

    if (hud_y == 0) return Raster_SetError(KQ_RASTER_BAD_RANGE);
    if (hud_y >= 144) return Raster_SetError(KQ_RASTER_BAD_RANGE);

    Raster_Clear();
    r = Raster_Push(0, camera_x, camera_y);
    if (r != KQ_RASTER_OK) return r;
    return Raster_Push(hud_y, 0, 0);
}

u8 Raster_BuildParallax2(u8 split_y, u8 far_scx, u8 near_scx, u8 scy)
{
    u8 r;

    if (split_y == 0) return Raster_SetError(KQ_RASTER_BAD_RANGE);
    if (split_y >= 144) return Raster_SetError(KQ_RASTER_BAD_RANGE);

    Raster_Clear();
    r = Raster_Push(0, far_scx, scy);
    if (r != KQ_RASTER_OK) return r;
    return Raster_Push(split_y, near_scx, scy);
}

u8 Raster_BuildParallax3(u8 y1, u8 y2, u8 scx0, u8 scx1, u8 scx2, u8 scy)
{
    u8 r;

    if (y1 == 0) return Raster_SetError(KQ_RASTER_BAD_RANGE);
    if (y2 >= 144) return Raster_SetError(KQ_RASTER_BAD_RANGE);
    if (y2 <= y1) return Raster_SetError(KQ_RASTER_BAD_RANGE);

    Raster_Clear();
    r = Raster_Push(0, scx0, scy);
    if (r != KQ_RASTER_OK) return r;
    r = Raster_Push(y1, scx1, scy);
    if (r != KQ_RASTER_OK) return r;
    return Raster_Push(y2, scx2, scy);
}

void Raster_LineXInit(RasterLineX* effect, u8 profile, u8 base_x)
{
    effect->profile = profile;
    effect->base_x = base_x;
    effect->phase = 0;
    effect->phase_step = 1;
    effect->frame_divider = 1;
    effect->frame_counter = 0;
}

void Raster_LineXSetProfile(RasterLineX* effect, u8 profile)
{
    effect->profile = profile;
    effect->phase = 0;
    effect->frame_counter = 0;
}

void Raster_LineXSetSpeed(RasterLineX* effect, u8 phase_step, u8 frame_divider)
{
    effect->phase_step = phase_step;
    if (frame_divider == 0) frame_divider = 1;
    effect->frame_divider = frame_divider;
    effect->frame_counter = 0;
}

void Raster_LineXSetPhase(RasterLineX* effect, u8 phase)
{
    effect->phase = phase;
}

u8 Raster_LineXGetOffset(const RasterLineX* effect, u8 ly)
{
    u8 index;

    if (effect->profile == KQ_RASTER_LINE_X_TITLE) {
        index = (u8)((ly + effect->phase) & 15);
        return (u8)(effect->base_x + Raster_LineXXTitle[(__safe_index u8)index]);
    }

    if (effect->profile == KQ_RASTER_LINE_TRAVEL_GATE) {
        index = (u8)((ly + effect->phase) & 63);
        return (u8)(effect->base_x + Raster_LineXTravelGate[(__safe_index u8)index]);
    }

    return effect->base_x;
}

void Raster_LineXRunFrame(RasterLineX* effect)
{
    u8 ly = 0;
    u8 scx;

    __wait_vblank();
    __scroll_bg_x_set(Raster_LineXGetOffset(effect, 0));

    while (ly < 144) {
        scx = Raster_LineXGetOffset(effect, ly);
        __wait_ly(ly);
        __scroll_bg_x_set(scx);
        ly++;
    }

    effect->frame_counter++;
    if (effect->frame_counter >= effect->frame_divider) {
        effect->frame_counter = 0;
        effect->phase = (u8)(effect->phase + effect->phase_step);
    }
}

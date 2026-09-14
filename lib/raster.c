#include "raster.h"

void __wait_ly(u8 ly);

// Bookkeeping tracks Raster_* submissions only. Mixing direct Scroll_Split* calls can invalidate these counts.
static u8 Raster_CountValue;
static u8 Raster_LastLyValue;
static u8 Raster_HasLastLy;
static u8 Raster_LastErrorValue;

// Wrapped byte offsets for the short title-wave profile; values near 255
// represent small negative displacements when added modulo 256.
static __prg_rom u8 Raster_LineXXTitle[16] = {
    252, 252, 253, 253, 254, 254, 255, 255,
    0, 0, 1, 1, 2, 2, 3, 3
};

// A longer symmetric horizontal-wave profile stored as wrapping byte offsets.
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

// Store and return the most recent raster API error code.
static u8 Raster_SetError(u8 error)
{
    Raster_LastErrorValue = error;
    return error;
}

// Require a visible scanline, free band capacity and strictly increasing LY.
// This validates without appending a band; failures update the error state.
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

// Initialize an empty pending split table and clear raster error/count state.
void Raster_Init()
{
    Raster_Clear();
}

// Reset pending splits and bookkeeping. Commit separately to replace active splits.
void Raster_Clear()
{
    Scroll_SplitReset();
    Raster_CountValue = 0;
    Raster_LastLyValue = 0;
    Raster_HasLastLy = 0;
    Raster_LastErrorValue = KQ_RASTER_OK;
}

// Commit an empty split table so no raster bands remain active.
void Raster_Disable()
{
    Raster_Clear();
    Scroll_SplitCommit();
}

// Read the number of bands currently recorded by this wrapper.
u8 Raster_GetCount()
{
    return Raster_CountValue;
}

// Read the last stored status without clearing it.
u8 Raster_GetLastError()
{
    return Raster_LastErrorValue;
}

// Append a validated background-scroll band and advance the monotonic-LY
// bookkeeping. A successful append clears the previous error.
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

// Append a validated extended band after masking flags to supported operations.
u8 Raster_PushEx(u8 ly, u8 scx, u8 scy, u8 wx, u8 wy, u8 flags)
{
    u8 r;

    r = Raster_CheckPush(ly);
    if (r != KQ_RASTER_OK) return r;

    // Masking removes unknown bits but does not reject conflicting supported operations.
    flags = (u8)(flags & KQ_RASTER_FLAG_MASK);
    Scroll_SplitPushEx(ly, scx, scy, wx, wy, flags);

    Raster_LastLyValue = ly;
    Raster_HasLastLy = 1;
    Raster_CountValue++;
    Raster_LastErrorValue = KQ_RASTER_OK;
    return KQ_RASTER_OK;
}

// Enable window updates and pass hardware WX/WY values without coordinate conversion.
u8 Raster_PushWindowRaw(u8 ly, u8 wx, u8 wy, u8 flags)
{
    flags = (u8)(flags | KQ_RASTER_WIN);
    return Raster_PushEx(ly, 0, 0, wx, wy, flags);
}

// Convert screen X to hardware WX by adding seven, then queue a window band.
// The byte addition wraps if callers supply an excessive X coordinate.
u8 Raster_PushWindowScreen(u8 ly, u8 screen_x, u8 screen_y, u8 flags)
{
    return Raster_PushWindowRaw(ly, (u8)(screen_x + 7), screen_y, flags);
}

// Queue both background and window updates, converting window screen X to WX.
u8 Raster_PushBgWindowScreen(u8 ly, u8 scx, u8 scy, u8 screen_x, u8 screen_y, u8 flags)
{
    flags = (u8)(flags | KQ_RASTER_BG | KQ_RASTER_WIN);
    return Raster_PushEx(ly, scx, scy, (u8)(screen_x + 7), screen_y, flags);
}

// Pack a 15-bit color into the extended band's two scroll-byte fields and select
// the color-zero operation, which interprets those fields as color data.
// This record reuses scroll payload bytes for color; do not combine the color operation with ordinary BG scroll data.
u8 Raster_PushBgColor0(u8 ly, u16 rgb15)
{
    return Raster_PushEx(ly, (u8)rgb15, (u8)(rgb15 >> 8), 0, 0, KQ_RASTER_BG_COLOR0);
}

// Publish pending splits and return the stored status. This commits even if an
// earlier append failed; callers should inspect errors while building the table.
u8 Raster_Commit()
{
    Scroll_SplitCommit();
    return Raster_LastErrorValue;
}

// Validate the HUD height, then replace pending splits with a fixed top band
// and scrolling playfield. The new table still requires an explicit commit.
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

// Build a scrolling upper playfield and fixed bottom HUD after validating the split.
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

// Build two horizontal-scroll bands sharing one vertical scroll value.
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

// Build three horizontal-scroll bands at ordered visible split lines. Invalid
// parameters leave the previous pending table intact and set the error status.
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

// Initialize a scanline-wave state with phase zero and one phase step per frame.
void Raster_LineXInit(RasterLineX* effect, u8 profile, u8 base_x)
{
    effect->profile = profile;
    effect->base_x = base_x;
    effect->phase = 0;
    effect->phase_step = 1;
    effect->frame_divider = 1;
    effect->frame_counter = 0;
}

// Change the wave profile and restart its phase/divider counter.
void Raster_LineXSetProfile(RasterLineX* effect, u8 profile)
{
    effect->profile = profile;
    effect->phase = 0;
    effect->frame_counter = 0;
}

// Configure the phase increment and frame interval; clamp a zero interval to one.
void Raster_LineXSetSpeed(RasterLineX* effect, u8 phase_step, u8 frame_divider)
{
    effect->phase_step = phase_step;
    if (frame_divider == 0) frame_divider = 1;
    effect->frame_divider = frame_divider;
    effect->frame_counter = 0;
}

// Set the byte phase directly without resetting the frame-divider counter.
void Raster_LineXSetPhase(RasterLineX* effect, u8 phase)
{
    effect->phase = phase;
}

// Look up the selected periodic profile at LY + phase and add base X modulo
// 256. An unknown profile returns the unmodified base X.
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

// Drive horizontal scroll through all 144 visible lines using blocking LY
// waits, then advance animation phase at the chosen frame interval. This owns
// the CPU for the effect frame and must not compete with another raster driver.
// The LCD must remain enabled for LY waits to progress. Delayed execution or interrupt work can miss a target line.
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

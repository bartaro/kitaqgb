/* Copyright (c) 2026 DAISUKE OBA. SPDX-License-Identifier: MIT.
   Shared constants, hardware registers, WRAM state and ROM graphics for the ordered gameplay
   translation unit. All following source fragments use these declarations. */

#pragma bank 2

/* build.ps1 concatenates the gameplay parts into one translation unit.
   These constants and static globals are shared by the following parts. */
#include "rpg.h"
#include "cgb_palette.h"
#include "cgb_tile.h"
#include "physics2d.h"
#include "chain.h"
#include "system.h"
#include "sprite.h"
#include "audio.h"
#include "audio_vblank.h"

#include "shirohebi_input_shared.h"
static void SND_ReturnToTitle();
static void SND_LockSystemButtons();
static void SND_DisableInputShortcuts();

#define SND_STATE_TITLE ((u8)0)
#define SND_STATE_PLAY  ((u8)1)
#define SND_STATE_MISS  ((u8)2)
#define SND_STATE_PAUSE ((u8)3)
#define SND_STATE_ERASE ((u8)4)
#define SND_STATE_GAMEOVER ((u8)5)
#define SND_STATE_READY ((u8)6)
#define SND_STATE_NAME_ENTRY ((u8)7)
#define SND_STATE_RETURN_CONFIRM ((u8)8)

#define SND_TILE_FONT_BASE ((u8)80)
#define SND_TILE_EXCL      ((u8)106)
#define SND_TILE_QUESTION  ((u8)107)
#define SND_TILE_ARROW     ((u8)108)
#define SND_TILE_HYPHEN    ((u8)109)
#define SND_TILE_HUD_BASE  ((u8)110)
#define SND_TILE_HUD_DIGIT ((u8)115)
#define SND_TILE_REV_DIGIT SND_TILE_HUD_DIGIT
#define SND_TILE_JP_TITLE  ((u8)125)
#define SND_JP_TITLE_TILE_COUNT ((u8)32)
#define SND_TILE_ASCII_LOWER ((u8)157)
#define SND_TILE_ASCII_SYMBOL ((u8)183)
#define SND_ASCII_LOWER_COUNT ((u8)26)
#define SND_ASCII_SYMBOL_COUNT ((u8)30)

#define SND_TILE_HEAD      ((u8)56)
#define SND_TILE_BODY      ((u8)57)
#define SND_TILE_TAIL      ((u8)58)
#define SND_TILE_APPLE     ((u8)59)
#define SND_TILE_HEAD_DOWN ((u8)70)
#define SND_TILE_HEAD_LEFT ((u8)71)
#define SND_TILE_HEAD_UP   ((u8)72)
#define SND_TILE_HEAD_DR   ((u8)73)
#define SND_TILE_HEAD_DL   ((u8)74)
#define SND_TILE_HEAD_UL   ((u8)75)
#define SND_TILE_HEAD_UR   ((u8)76)
#define SND_TILE_FLASH     ((u8)77)
#define SND_TILE_BURST     ((u8)78)
#define SND_TILE_SCORCH    ((u8)79)

#define SND_TILE_FLOOR_DOT ((u8)1)

/* Segment counts include the head at index zero; up to 30 joints follow it. */
#define SND_MAX_SEGMENTS      ((u8)31)
#define SND_START_BODY_COUNT  ((u8)3)
#define SND_START_LENGTH      ((u8)(SND_START_BODY_COUNT + 1))
#define SND_APPLE_COUNT       ((u8)16)
#define SND_BLOCK_COUNT       ((u8)2)
#define SND_TRACE_COUNT       ((u8)32)
#define SND_SEGMENT_GAP       ((u8)4)
#define SND_JOINT_FAST_GAP    ((u8)7)
#define SND_SELF_HIT_SKIP     ((u8)2)
#define SND_SELF_HIT_RADIUS   ((u8)3)
#define SND_BLOCK_HIT_RADIUS  ((u8)7)
#define SND_BLOCK_MIN_Y       ((u8)62)
#define SND_BLOCK_MAX_Y       ((u8)74)
#define SND_BLOCK_LEFT_MIN_X  ((u8)46)
#define SND_BLOCK_LEFT_MAX_X  ((u8)58)
#define SND_BLOCK_RIGHT_MIN_X ((u8)102)
#define SND_BLOCK_RIGHT_MAX_X ((u8)114)
#define SND_OAM_SNAKE_BASE    ((u8)0)
#define SND_OAM_PLAY_USED     ((u8)31)
#define SND_OAM_SNAKE_FIXED_FRONT ((u8)6)
#define SND_OAM_SNAKE_ROTATE_STEP ((u8)4)
#define SND_OAM_TEXT_BASE     ((u8)31)
#define SND_OAM_TEXT_COUNT    ((u8)9)
#define SND_BG_APPLE_ATTR     KQ_CGB_ATTR_PAL(5)
#define SND_BG_SCORCH_ATTR    KQ_CGB_ATTR_PAL(6)
#define SND_OBJ_TEXT_ATTR     ((u8)0x12)
#define SND_OBJ_HIT_ATTR      KQ_CGB_ATTR_PAL(3)
#define SND_OBJ_GROW_ATTR     KQ_CGB_ATTR_PAL(4)
#define SND_OBJ_DMG_PAL1      ((u8)0x10)
#define SND_HISCORE_COUNT     ((u8)10)
#define SND_OLD_SCORE_COUNT   ((u16)5)
#define SND_LEGACY_SCORE_BYTES ((u16)15)
#define SND_NAME_LENGTH       ((u8)3)
#define SND_NAME_BYTES        ((u8)30)
#define SND_NAME_CHAR_COUNT   ((u8)27)
#define SND_SAVE_MAGIC        ((u8)0x53)
#define SND_SAVE_VERSION      ((u8)2)
#define SND_SAVE_SCORE_OFFSET ((u8)2)
#define SND_SAVE_NAME_OFFSET  ((u8)12)
#define SND_SAVE_DATA_BYTES   ((u16)42)
#define SND_PENDING_NONE      ((u8)0xFF)
#define SND_PAUSE_X           ((u8)7)
#define SND_PAUSE_Y           ((u8)8)
#define SND_PAUSE_LEN         ((u8)5)
#define SND_START_LABEL_X     ((u8)7)
#define SND_START_LABEL_Y     ((u8)6)
#define SND_START_LABEL_LEN   ((u8)5)
#define SND_START_FLASH_ON    ((u8)22)
#define SND_START_FLASH_CYCLE ((u8)44)
#define SND_START_FLASH_TOTAL ((u8)176)
#define SND_TITLE_MENU_COUNT  ((u8)2)
#define SND_TITLE_MENU_MUSIC  ((u8)0)
#define SND_TITLE_MENU_SOUND  ((u8)1)
#define SND_TITLE_PROMPT_X    ((u8)1)
#define SND_TITLE_PROMPT_Y    ((u8)16)
#define SND_TITLE_PROMPT_LEN  ((u8)18)
#define SND_TILE_COPYRIGHT_BASE ((u8)213)
#define SND_COPYRIGHT_TILE_COUNT ((u8)19)
#define SND_COPYRIGHT_X       ((u8)1)
#define SND_COPYRIGHT_Y       ((u8)17)
#define SND_TITLE_BLINK_MASK  ((u8)64)
#define SND_TITLE_DEMO_WAIT_FRAMES ((u16)900)
#define SND_DEMO_PLAY_SECONDS ((u8)30)
#define SND_DEMO_VBLANK_TICKS_PER_SECOND ((u8)60)
#define SND_DEMO_LABEL_X     ((u8)15)
#define SND_DEMO_LABEL_Y     ((u8)0)
#define SND_ERASE_NO          ((u8)0)
#define SND_ERASE_YES         ((u8)1)
#define SND_COMMAND_BUTTONS ((u8)(PAD_KEY_A | PAD_KEY_B | PAD_KEY_SELECT | PAD_KEY_START))
#define SND_TITLE_SHORTCUT ((u8)(PAD_KEY_SELECT | PAD_KEY_START))
#define SND_RETURN_NO         ((u8)0)
#define SND_RETURN_YES        ((u8)1)
#define SND_GAMEOVER_YES      ((u8)0)
#define SND_GAMEOVER_NO       ((u8)1)
#define SND_MINE_NONE         ((u8)0xFF)
#define SND_SEGMENT_NONE      ((u8)0xFF)
#define SND_COLLISION_NONE    ((u8)0)
#define SND_COLLISION_MINE    ((u8)1)
#define SND_COLLISION_SELF    ((u8)2)
#define SND_GROW_FLASH_FRAMES ((u8)20)
#define SND_DIR_RIGHT         ((u8)0)
#define SND_DIR_DOWN          ((u8)4)
#define SND_DIR_LEFT          ((u8)8)
#define SND_DIR_UP            ((u8)12)
#define SND_DIR_STEPS         ((u8)16)
#define SND_REPEAT_FIRST      ((u8)8)
#define SND_REPEAT_NEXT       ((u8)4)
#define SND_STEER_NONE        ((u8)0)
#define SND_STEER_LEFT        ((u8)1)
#define SND_STEER_RIGHT       ((u8)2)

/* Physics positions/velocities use Q4 units (16 units per pixel).
   Trace coordinates and collision radii use whole pixels. */
#define SND_PHYS_SHIFT        ((u8)4)
#define SND_START_VEL_Q4      ((s16)4)
#define SND_BASE_MAX_VEL_Q4   ((s16)8)
#define SND_SPEED_STEP_Q4     ((s16)2)
#define SND_SPEED_STAGE_MAX   ((u8)10)
#define SND_ACCEL_Q4          ((s16)2)
#define SND_ALIGN_STEP_Q4     ((s16)3)
#define SND_BOOST_STEP_MIN_Q4 ((s16)1)
#define SND_MISS_FLASH_FRAMES ((u8)8)
#define SND_MISS_BURST_FRAME  ((u8)44)
#define SND_MISS_CLEAR_FRAME  ((u8)72)
#define SND_MISS_BURST_LEN    ((u8)10)
#define SND_SELF_FADE_LIGHT_FRAME ((u8)8)
#define SND_SELF_FADE_DARK_FRAME  ((u8)24)
#define SND_SELF_FADE_BLUE_FRAME  ((u8)40)
#define SND_SELF_FADE_HIDE_FRAME  ((u8)64)
#define SND_DMG_FADE_LIGHT        ((u8)0x50)
#define SND_DMG_FADE_DARK         ((u8)0xA0)
#define SND_DMG_FADE_BLUE         ((u8)0xF0)
#define SND_START_X           ((u8)80)
#define SND_START_Y           ((u8)68)
#define SND_START_X_Q4        ((s16)1280)
#define SND_START_Y_Q4        ((s16)1088)
#define SND_BLOCK_LEFT_X      ((u8)52)
#define SND_BLOCK_RIGHT_X     ((u8)108)
#define SND_BLOCK_CENTER_Y    ((u8)68)
#define SND_HALF_SIZE_Q4      ((s16)64)
#define SND_WRAP_MIN_X_Q4     ((s16)64)
#define SND_WRAP_MAX_X_Q4     ((s16)2496)
#define SND_WRAP_MIN_Y_Q4     ((s16)64)
#define SND_WRAP_MAX_Y_Q4     ((s16)2240)
#define SND_SCREEN_W          ((u8)160)
#define SND_SCREEN_H          ((u8)144)
#define SND_AUDIO_WORK_MASK   ((u8)0)
#define SND_AUDIO_VBLANK_LINE ((u8)144)

extern __prg_rom u8 shirohebi_map_tiles[];
extern __prg_rom u8 shirohebi_map_attrs[];
void shirohebi_upload_tiles();

__location(0xFF40) u8 SND_LCDC;
__location(0xFF44) u8 SND_LY;
__location(0xFF47) u8 SND_BGP;
__location(0xFF48) u8 SND_OBP0;
__location(0xFF49) u8 SND_OBP1;

/* Despite the name, Trace stores each joint's current pose, not a head-history
   ring buffer. TraceDir delays heading changes through successive joints. */
__wram u8 SND_TraceX[SND_TRACE_COUNT];
__wram u8 SND_TraceY[SND_TRACE_COUNT];
__wram u8 SND_TraceDir[SND_TRACE_COUNT];
static __wram ChainBody SND_ChainBody;
__wram u8 SND_AppleActive[SND_APPLE_COUNT];
__wram s16 SND_MissXQ4[SND_TRACE_COUNT];
__wram s16 SND_MissYQ4[SND_TRACE_COUNT];
__wram s16 SND_MissVXQ4[SND_TRACE_COUNT];
__wram s16 SND_MissVYQ4[SND_TRACE_COUNT];
static __wram KQBody2D SND_HeadBody;
static __wram KQWorld2D SND_World;

static u8 SND_State;
static u8 SND_PrevKeys;
static u8 SND_Length;
static u8 SND_IsCgb;
static u8 SND_AppleIndex;
static u8 SND_MissTimer;
static u8 SND_ApplesEaten;
static u8 SND_Facing;
static u8 SND_PauseTimer;
static u8 SND_PauseShown;
static u8 SND_PauseDesired;
static u8 SND_PauseDirty;
static u8 SND_StartTimer;
static u8 SND_StartShown;
static u8 SND_StartDesired;
static u8 SND_StartDirty;
static u8 SND_TurnRepeatDir;
static u8 SND_TurnRepeatTimer;
static u8 SND_CollisionSegment;
static u8 SND_CollisionSegment2;
static u8 SND_CollisionKind;
static u8 SND_TitleMenuPos;
static u8 SND_TitlePromptTimer;
static u8 SND_TitlePromptShown;
static u16 SND_TitleIdleFrames;
static u8 SND_DemoActive;
static u8 SND_DemoScene;
static u8 SND_DemoWeaveStep;
static u8 SND_DemoSeconds;
static u8 SND_DemoSubTicks;
static u8 SND_DemoVBlankPrevious;
static u8 SND_MusicOn;
static u8 SND_SoundOn;
static u8 SND_UpBoostHeld;
static u8 SND_EraseChoice;
/* A transition consumes input until every button has been released. */
static u8 SND_InputReleaseLock;
static u8 SND_ReturnChoice;
static u8 SND_TailOamPhase;
static u8 SND_GameOverChoice;
static u8 SND_HitMineIndex;
static s16 SND_CurrentMaxVelQ4;
static u8 SND_GrowFlashSegment;
static u8 SND_GrowFlashTimer;
static __wram u8 SND_HighScores[SND_HISCORE_COUNT];
static __wram u8 SND_HighScoreNames[SND_NAME_BYTES];
static __wram u8 SND_SaveData[SND_SAVE_DATA_BYTES];
static __wram u8 SND_LegacyScores[SND_LEGACY_SCORE_BYTES];
static __wram u8 SND_NameIndices[SND_NAME_LENGTH];
static u8 SND_PendingRank;
static u8 SND_NameCursor;
u8 SND_HeadX;
u8 SND_HeadY;
static u8 SND_ApplePosX;
static u8 SND_ApplePosY;

/* Apple/Block identifiers refer to pomegranates/mines respectively.
   Candidate fruit centers are visited cyclically, not selected randomly. */
static __prg_rom u8 SND_AppleX[SND_APPLE_COUNT] = {
    20, 140, 84, 20, 140, 28, 132, 84,
    76, 92, 68, 92, 60, 100, 52, 108
};
static __prg_rom u8 SND_AppleY[SND_APPLE_COUNT] = {
    20, 20, 28, 72, 72, 124, 124, 124,
    44, 44, 84, 84, 68, 68, 108, 108
};
static __prg_rom u8 SND_BlockX[SND_BLOCK_COUNT] = {
    SND_BLOCK_LEFT_X, SND_BLOCK_RIGHT_X
};
static __prg_rom u8 SND_BlockY[SND_BLOCK_COUNT] = {
    SND_BLOCK_CENTER_Y, SND_BLOCK_CENTER_Y
};

static __prg_rom u16 SND_BG_PAL_TEXT[4] = {
    CGB_RGB15(2, 4, 8), CGB_RGB15(10, 14, 20),
    CGB_RGB15(23, 25, 26), CGB_RGB15(31, 31, 31)
};
static __prg_rom u16 SND_BG_PAL_FLOOR[4] = {
    CGB_RGB15(2, 8, 4), CGB_RGB15(8, 19, 10),
    CGB_RGB15(17, 26, 15), CGB_RGB15(29, 31, 22)
};
static __prg_rom u16 SND_BG_PAL_DOT[4] = {
    CGB_RGB15(2, 8, 4), CGB_RGB15(8, 19, 10),
    CGB_RGB15(24, 13, 29), CGB_RGB15(30, 24, 31)
};
static __prg_rom u16 SND_BG_PAL_APPLE[4] = {
    CGB_RGB15(2, 8, 4), CGB_RGB15(8, 19, 10),
    CGB_RGB15(31, 10, 8), CGB_RGB15(31, 10, 8)
};
static __prg_rom u16 SND_BG_PAL_SCORCH[4] = {
    CGB_RGB15(2, 8, 4), CGB_RGB15(10, 8, 3),
    CGB_RGB15(17, 10, 4), CGB_RGB15(4, 3, 2)
};
static __prg_rom u16 SND_BG_PAL_COPYRIGHT_GOLD[4] = {
    CGB_RGB15(2, 8, 4), CGB_RGB15(8, 19, 10),
    CGB_RGB15(17, 26, 15), CGB_RGB15(31, 28, 13)
};
static __prg_rom u16 SND_BG_PAL_COPYRIGHT_IVORY[4] = {
    CGB_RGB15(2, 8, 4), CGB_RGB15(8, 19, 10),
    CGB_RGB15(17, 26, 15), CGB_RGB15(31, 31, 28)
};
static __prg_rom u16 SND_BG_PAL_COPYRIGHT_PINK[4] = {
    CGB_RGB15(2, 8, 4), CGB_RGB15(8, 19, 10),
    CGB_RGB15(17, 26, 15), CGB_RGB15(30, 20, 25)
};
static __prg_rom u16 SND_OBJ_PAL_SNAKE[4] = {
    CGB_RGB15(0, 0, 0), CGB_RGB15(31, 0, 0),
    CGB_RGB15(31, 31, 31), CGB_RGB15(30, 31, 31)
};
static __prg_rom u16 SND_OBJ_PAL_APPLE[4] = {
    CGB_RGB15(0, 0, 0), CGB_RGB15(14, 6, 2),
    CGB_RGB15(31, 10, 8), CGB_RGB15(31, 10, 8)
};
static __prg_rom u16 SND_OBJ_PAL_HIT[4] = {
    CGB_RGB15(0, 0, 0), CGB_RGB15(16, 0, 0),
    CGB_RGB15(31, 0, 0), CGB_RGB15(31, 12, 8)
};
static __prg_rom u16 SND_OBJ_PAL_GROW[4] = {
    CGB_RGB15(0, 0, 0), CGB_RGB15(20, 15, 0),
    CGB_RGB15(31, 28, 4), CGB_RGB15(31, 31, 18)
};
static __prg_rom u16 SND_OBJ_PAL_TEXT[4] = {
    CGB_RGB15(0, 0, 0), CGB_RGB15(10, 14, 20),
    CGB_RGB15(23, 25, 26), CGB_RGB15(31, 31, 31)
};
static __prg_rom u16 SND_OBJ_PAL_FADE_LIGHT[4] = {
    CGB_RGB15(0, 0, 0), CGB_RGB15(23, 23, 23),
    CGB_RGB15(23, 23, 23), CGB_RGB15(23, 23, 23)
};
static __prg_rom u16 SND_OBJ_PAL_FADE_DARK[4] = {
    CGB_RGB15(0, 0, 0), CGB_RGB15(11, 11, 11),
    CGB_RGB15(11, 11, 11), CGB_RGB15(11, 11, 11)
};
static __prg_rom u16 SND_OBJ_PAL_FADE_BLUE[4] = {
    CGB_RGB15(0, 0, 0), CGB_RGB15(2, 7, 28),
    CGB_RGB15(2, 7, 28), CGB_RGB15(2, 7, 28)
};
/* Runtime snake, fruit and effect graphics. The Asset Studio export provides
   grass/mines separately; these 24 tiles occupy VRAM tile IDs 56..79. */
static __prg_rom u8 SND_SPRITE_TILES[384] = {
    0x00, 0x38, 0x38, 0x7C, 0x7C, 0xFA, 0x7C, 0xFE,
    0x7C, 0xFA, 0x38, 0x7C, 0x00, 0x38, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x3C,
    0x18, 0x3C, 0x00, 0x18, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x08, 0x1C,
    0x00, 0x18, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00,
    0xEF, 0x10, 0xC7, 0x38, 0x83, 0x7C, 0x81, 0x7E,
    0x81, 0x7E, 0xC3, 0x3C, 0xE7, 0x18, 0xFF, 0x00,
    0x00, 0x70, 0x00, 0x88, 0x00, 0x88, 0x00, 0x88,
    0x00, 0x88, 0x00, 0x88, 0x00, 0x70, 0x00, 0x00,
    0x00, 0x20, 0x00, 0x60, 0x00, 0x20, 0x00, 0x20,
    0x00, 0x20, 0x00, 0x20, 0x00, 0x70, 0x00, 0x00,
    0x00, 0x70, 0x00, 0x88, 0x00, 0x08, 0x00, 0x10,
    0x00, 0x20, 0x00, 0x40, 0x00, 0xF8, 0x00, 0x00,
    0x00, 0xF0, 0x00, 0x08, 0x00, 0x10, 0x00, 0x30,
    0x00, 0x08, 0x00, 0x88, 0x00, 0x70, 0x00, 0x00,
    0x00, 0x10, 0x00, 0x30, 0x00, 0x50, 0x00, 0x90,
    0x00, 0xF8, 0x00, 0x10, 0x00, 0x10, 0x00, 0x00,
    0x00, 0xF8, 0x00, 0x80, 0x00, 0xF0, 0x00, 0x08,
    0x00, 0x08, 0x00, 0x88, 0x00, 0x70, 0x00, 0x00,
    0x00, 0x70, 0x00, 0x80, 0x00, 0x80, 0x00, 0xF0,
    0x00, 0x88, 0x00, 0x88, 0x00, 0x70, 0x00, 0x00,
    0x00, 0xF8, 0x00, 0x08, 0x00, 0x10, 0x00, 0x20,
    0x00, 0x40, 0x00, 0x40, 0x00, 0x40, 0x00, 0x00,
    0x00, 0x70, 0x00, 0x88, 0x00, 0x88, 0x00, 0x70,
    0x00, 0x88, 0x00, 0x88, 0x00, 0x70, 0x00, 0x00,
    0x00, 0x70, 0x00, 0x88, 0x00, 0x88, 0x00, 0x78,
    0x00, 0x08, 0x00, 0x08, 0x00, 0x70, 0x00, 0x00,
    0x00, 0x38, 0x38, 0x7C, 0x7C, 0xFE, 0x7C, 0xFE,
    0x7C, 0xFE, 0x38, 0x54, 0x00, 0x38, 0x00, 0x00,
    0x00, 0x1C, 0x1C, 0x3E, 0x3E, 0x5F, 0x3E, 0x7F,
    0x3E, 0x5F, 0x1C, 0x3E, 0x00, 0x1C, 0x00, 0x00,
    0x00, 0x38, 0x38, 0x54, 0x7C, 0xFE, 0x7C, 0xFE,
    0x7C, 0xFE, 0x38, 0x7C, 0x00, 0x38, 0x00, 0x00,
    0x00, 0x38, 0x38, 0x7C, 0x7C, 0xFC, 0x7C, 0xFE,
    0x3E, 0x7D, 0x1C, 0x3A, 0x00, 0x1C, 0x00, 0x00,
    0x00, 0x1C, 0x1C, 0x3E, 0x3E, 0x3F, 0x3E, 0x7F,
    0x7C, 0xBE, 0x38, 0x5C, 0x00, 0x38, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x38, 0x38, 0x5C, 0x7C, 0xBE,
    0x3E, 0x7F, 0x3E, 0x3F, 0x1C, 0x3E, 0x00, 0x1C,
    0x00, 0x00, 0x00, 0x1C, 0x1C, 0x3A, 0x3E, 0x7D,
    0x7C, 0xFE, 0x7C, 0xFC, 0x38, 0x7C, 0x00, 0x38,
    0x00, 0x00, 0x10, 0x38, 0x38, 0x7C, 0x7C, 0xFE,
    0x7C, 0xFE, 0x38, 0x7C, 0x10, 0x38, 0x00, 0x00,
    0x10, 0x10, 0x44, 0x44, 0x28, 0x28, 0x82, 0x82,
    0x44, 0x44, 0x28, 0x28, 0x92, 0x92, 0x00, 0x00,
    0x00, 0x3C, 0x18, 0x7E, 0x24, 0x7E, 0x5A, 0xFF,
    0x3C, 0xFF, 0x24, 0x7E, 0x18, 0x7E, 0x00, 0x3C,
};

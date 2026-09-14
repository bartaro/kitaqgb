#pragma once

typedef void (*SceneFunc)();

// Callbacks may be null. Keep the table and non-null targets readable for its lifetime.
// These plain function pointers do not carry a ROM bank number.
typedef __packed struct {
    SceneFunc enter;
    SceneFunc update;
    SceneFunc draw;
    SceneFunc exit;
} SceneDef;

// Attach the caller-owned scene table and clear current-scene state without invoking callbacks.
void scene_init(const SceneDef* scenes, u8 count);
// Replace the table by resetting scene state; this does not call the old scene's exit handler.
void scene_set_table(const SceneDef* scenes, u8 count);
// Ignore invalid IDs; otherwise exit the current scene and enter the requested one.
// Selecting the current ID still performs exit/enter. Callbacks run synchronously.
void scene_change(u8 scene_id);
// Clear the change flag before calling the active scene's update handler, so a
// transition made during that handler is visible afterward.
void scene_update();
// Call the active scene's draw handler if one exists; no current scene is a no-op.
void scene_draw();
// Return the stored scene ID; zero is also returned before the first transition.
u8 scene_get_current();
// Read the transition flag, which is cleared at the start of an active scene update.
u8 scene_was_changed();


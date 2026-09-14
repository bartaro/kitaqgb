#include "scene.h"

static const SceneDef* scene_table;
static u8 scene_count;
static u8 scene_current;
static u8 scene_changed;
static u8 scene_has_current;

// Attach the caller-owned scene table and clear current-scene state without invoking callbacks.
void scene_init(const SceneDef* scenes, u8 count)
{
    scene_table = scenes;
    scene_count = count;
    scene_current = 0;
    scene_changed = 0;
    scene_has_current = 0;
}

// Replace the table by resetting scene state; this does not call the old scene's exit handler.
void scene_set_table(const SceneDef* scenes, u8 count)
{
    scene_init(scenes, count);
}

// Ignore invalid IDs; otherwise exit the current scene and enter the requested one.
// Selecting the current ID still performs exit/enter. Callbacks run synchronously.
void scene_change(u8 scene_id)
{
    SceneFunc fn;

    if (scene_id >= scene_count) return;

    // Exit/enter handlers must not recursively change scenes or replace this shared table.
    if (scene_has_current != 0) {
        fn = scene_table[(__safe_index u8)scene_current].exit;
        if (fn != 0) fn();
    }

    scene_current = scene_id;
    scene_has_current = 1;
    scene_changed = 1;

    fn = scene_table[(__safe_index u8)scene_current].enter;
    if (fn != 0) fn();
}

// Clear the change flag before calling the active scene's update handler, so a
// transition made during that handler is visible afterward.
void scene_update()
{
    SceneFunc fn;

    if (scene_has_current == 0) return;
    scene_changed = 0;
    fn = scene_table[(__safe_index u8)scene_current].update;
    if (fn != 0) fn();
}

// Call the active scene's draw handler if one exists; no current scene is a no-op.
void scene_draw()
{
    SceneFunc fn;

    if (scene_has_current == 0) return;
    fn = scene_table[(__safe_index u8)scene_current].draw;
    if (fn != 0) fn();
}

// Return the stored scene ID; zero is also returned before the first transition.
u8 scene_get_current()
{
    return scene_current;
}

// Read the transition flag, which is cleared at the start of an active scene update.
u8 scene_was_changed()
{
    return scene_changed;
}

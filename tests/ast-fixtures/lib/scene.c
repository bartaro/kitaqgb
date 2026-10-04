#include "scene.h"

static const SceneDef* scene_table;
static u8 scene_count;
static u8 scene_current;
static u8 scene_changed;
static u8 scene_has_current;

void scene_init(const SceneDef* scenes, u8 count)
{
    scene_table = scenes;
    scene_count = count;
    scene_current = 0;
    scene_changed = 0;
    scene_has_current = 0;
}

void scene_set_table(const SceneDef* scenes, u8 count)
{
    scene_init(scenes, count);
}

void scene_change(u8 scene_id)
{
    SceneFunc fn;

    if (scene_id >= scene_count) return;

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

void scene_update()
{
    SceneFunc fn;

    if (scene_has_current == 0) return;
    scene_changed = 0;
    fn = scene_table[(__safe_index u8)scene_current].update;
    if (fn != 0) fn();
}

void scene_draw()
{
    SceneFunc fn;

    if (scene_has_current == 0) return;
    fn = scene_table[(__safe_index u8)scene_current].draw;
    if (fn != 0) fn();
}

u8 scene_get_current()
{
    return scene_current;
}

u8 scene_was_changed()
{
    return scene_changed;
}

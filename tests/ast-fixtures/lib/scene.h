#pragma once

typedef void (*SceneFunc)();

typedef __packed struct {
    SceneFunc enter;
    SceneFunc update;
    SceneFunc draw;
    SceneFunc exit;
} SceneDef;

void scene_init(const SceneDef* scenes, u8 count);
void scene_set_table(const SceneDef* scenes, u8 count);
void scene_change(u8 scene_id);
void scene_update();
void scene_draw();
u8 scene_get_current();
u8 scene_was_changed();


// Switch the current scene ID.
// Expected: 001; no callbacks registered
#include "gb_common.h"
#include "scene.h"
SceneDef manual_scenes[2];
// Register two zero-initialized scene entries, select ID 1 and display it.
// No callbacks are registered in this example.
void main() {
    m_init();
    m_text(2,3,"SCENE");
    scene_init(manual_scenes,2); scene_change(1);
    m_number(scene_get_current());
    while (1) { m_wait();  }
}

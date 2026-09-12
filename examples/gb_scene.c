// シーン番号の切替
// Expected: 001; no callbacks registered
#include "gb_common.h"
#include "scene.h"
SceneDef manual_scenes[2];
void main() {
    m_init();
    m_text(2,3,"SCENE");
    scene_init(manual_scenes,2); scene_change(1);
    m_number(scene_get_current());
    while (1) { m_wait();  }
}

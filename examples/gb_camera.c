// 世界座標から画面座標へ
// Expected: 042: world x=52 minus camera x=10
#include "gb_common.h"
#include "camera.h"
Camera8_8 cam;
void main() {
    m_init();
    m_text(2,3,"CAMERA");
    Camera_Init(&cam); Camera_Set(&cam,2560,0);
    m_number((u8)Camera_WorldToScreenX(&cam,52));
    while (1) { m_wait();  }
}

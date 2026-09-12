// 通信が届かない場合の扱い
// Expected: 255 for disconnected link; 000 if timeout; peer value when connected
#include "gb_common.h"
#include "link.h"

void main() {
    m_init();
    m_text(2,3,"LINK");
    u8 value; Link_InitMaster(); Link_BeginTransfer(42);
    if(Link_WaitByte(4,&value)) m_number(value);
    else { Link_Cancel(); m_number(0); }
    while (1) { m_wait();  }
}

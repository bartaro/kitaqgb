// Handle serial transfers that do not deliver a byte in time.
// Expected: 255 for disconnected link; 000 if timeout; peer value when connected
#include "gb_common.h"
#include "link.h"

// Start a master transfer of byte 42 and wait with a bounded timeout.
// Display the received byte on completion; cancel and display zero on timeout.
void main() {
    m_init();
    m_text(2,3,"LINK");
    u8 value; Link_InitMaster(); Link_BeginTransfer(42);
    if(Link_WaitByte(4,&value)) m_number(value);
    else { Link_Cancel(); m_number(0); }
    while (1) { m_wait();  }
}

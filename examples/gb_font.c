// Display the letters, digits and punctuation from the supplied ascii.c font.
// Expected: Uppercase, lowercase, numbers and all 30 supplied punctuation glyphs
#include "gb_common.h"


// Lay out every supplied character category, yielding between rows. The final
// m_put uses character code 34 to display the double quote separately.
void main() {
    m_init();
    m_text(2,3,"FONT");
    m_text(2,5,"ABCDEFGHIJKLMNOP"); m_wait();
    m_text(2,6,"QRSTUVWXYZ"); m_wait();
    m_text(2,8,"0123456789"); m_wait();
    m_text(2,10,"abcdefghijklmnop"); m_wait();
    m_text(2,11,"qrstuvwxyz"); m_wait();
    m_text(2,13,"!#$%&'()*+,-./"); m_wait();
    m_text(2,14,":;<=>?@[]^_`{}~"); m_wait();
    m_put(16,13,34); m_wait();
    while (1) { m_wait();  }
}

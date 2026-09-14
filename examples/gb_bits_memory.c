// Bit operations and memory copying.
// Expected: 010
#include "gb_common.h"
void __memset(void* dst,u8 value,u16 len);
void __memcpy(void* dst,const void* src,u16 len);
void __bit_set(u8* base,u16 bit);
void __bit_toggle(u8* base,u16 bit);

// Clear the source bytes, set bits 3 and 1, copy the buffer, and display 8 + 2 = 10.
void main() {
    m_init();
    m_text(2,3,"BITS MEMORY");
    u8 data[4]; u8 copy[4];
    __memset(data,0,4); __bit_set(data,3); __bit_toggle(data,1);
    __memcpy(copy,data,4); m_number(copy[0]);
    while (1) { m_wait();  }
}

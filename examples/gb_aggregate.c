// Functions, arrays, structures and pointers.
// Expected: 042
#include "gb_common.h"

typedef struct { u8 x; u8 y; } Point;
// Add two byte values and explicitly narrow the returned result to u8.
u8 add(u8 a,u8 b) { return (u8)(a+b); }
// Copy a structure, fill an array through a pointer, and combine selected values
// through add() to produce 42. Keep the final screen visible in the idle loop.
void main() {
    m_init();
    m_text(2,3,"AGGREGATE");
    Point a; Point b; u8 i; u8 data[4]; u8* ptr;
    a.x=10; a.y=20; b=a; ptr=data;
    for(i=0;i<4;i++) ptr[i]=(u8)(i+1);
    m_number((u8)(add(b.x,b.y)+ptr[0]+ptr[1]+ptr[3]+5));
    while (1) { m_wait();  }
}

// 関数・配列・構造体・ポインタ
// Expected: 042
#include "gb_common.h"

typedef struct { u8 x; u8 y; } Point;
u8 add(u8 a,u8 b) { return (u8)(a+b); }
void main() {
    m_init();
    m_text(2,3,"AGGREGATE");
    Point a; Point b; u8 i; u8 data[4]; u8* ptr;
    a.x=10; a.y=20; b=a; ptr=data;
    for(i=0;i<4;i++) ptr[i]=(u8)(i+1);
    m_number((u8)(add(b.x,b.y)+ptr[0]+ptr[1]+ptr[3]+5));
    while (1) { m_wait();  }
}

#define CONST 77
void f(){ __asm {
LD A,#CONST
.local:
JR .local
} }
int a=CONST;

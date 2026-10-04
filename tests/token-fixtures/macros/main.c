#define A 3
#define B (A+7)
#define F(x,y) ((x)+(y))
#define G(x) F(x,B)
#define R S
#define S R
#define EMPTY()
int a=G(F(1,2)); int b=R; EMPTY() int c=B;
#undef A
int d=A;

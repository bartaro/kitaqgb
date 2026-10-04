#define A 7
#define F(x) x
#if defined(A) && (A<<1)==14 && !defined(NO)
int selected=1;
#elif 1/0
int excluded=0;
#else
int excluded2=0;
#endif
#if 0 && (1/0) || 1 || (3%0)
int ok=1;
#endif
#ifndef NO
#if ~0==-1 && 9/2==4 && 9%2==1 && (1<<65)==2 && undefined==0 && defined F
int nested=1;
#endif
#endif

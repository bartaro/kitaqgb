struct Tiny {u8 a;};
struct Pair {u16 a;};
struct Short {u8 data[15];};
struct Medium {u8 data[19];};
struct Wide {u8 data[80];};
struct Large {u8 data[260];};
struct Holder {struct Short item;u16 value;};
struct Tiny t1;struct Tiny t2;struct Pair p1;struct Pair p2;
struct Short s1;struct Short s2;struct Medium m1;struct Medium m2;
struct Wide w1;struct Wide w2;struct Large l1;struct Large l2;
struct Short objects[2];struct Short* pointer;struct Holder holder;
void main(){
    t1=t2;p1=p2;s1=s2;m1=m2;w1=w2;l1=l2;
    objects[1]=objects[0];pointer=&s1;*pointer=s2;
    holder.item=s1;holder.item.data[3]=7;holder.value=300;
}

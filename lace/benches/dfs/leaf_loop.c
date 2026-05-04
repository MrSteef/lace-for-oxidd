extern int GRAIN;
int __attribute__((noinline)) leaf_loop()
{
    int i, s=0;
    for( i=0; i<GRAIN; i++ ) {
        s += i;
        s *= i;
        s ^= i;
        s *= i;
        s += i;
    }
    return s;
}

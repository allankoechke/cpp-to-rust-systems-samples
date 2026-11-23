#include <stdint.h>
#include <stdlib.h>
typedef struct counter { int64_t value; } counter;
counter *counter_new(int64_t start) { counter *c = malloc(sizeof *c); if (c) c->value = start; return c; }
int64_t counter_incr(counter *c) { return ++c->value; }
void counter_free(counter *c) { free(c); }

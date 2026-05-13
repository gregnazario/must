#include "util.h"
#include <math.h>

int add(int a, int b) {
    return a + b;
}

double average(int *arr, int len) {
    if (len <= 0) return 0.0;
    double sum = 0.0;
    for (int i = 0; i < len; i++) {
        sum += arr[i];
    }
    return sum / len;
}

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "util.h"

int run_self_test(void) {
    int result = add(2, 3);
    if (result != 5) {
        fprintf(stderr, "FAIL: add(2,3) = %d, expected 5\n", result);
        return 1;
    }

    int nums[] = {10, 20, 30};
    double avg = average(nums, 3);
    if (fabs(avg - 20.0) > 0.001) {
        fprintf(stderr, "FAIL: average = %f, expected 20.0\n", avg);
        return 1;
    }

    printf("All tests passed.\n");
    return 0;
}

int main(int argc, char **argv) {
    if (argc > 1 && strcmp(argv[1], "--self-test") == 0) {
        return run_self_test();
    }

    int values[] = {4, 8, 15, 16, 23, 42};
    int n = sizeof(values) / sizeof(values[0]);

    printf("Sum of first two: %d\n", add(values[0], values[1]));
    printf("Average of all: %.2f\n", average(values, n));

    return 0;
}

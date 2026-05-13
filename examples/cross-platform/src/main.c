#include <stdio.h>

int main(void) {
    printf("cross-platform-demo v0.1.0\n");
#ifdef _WIN32
    printf("Platform: Windows\n");
#elif __APPLE__
    printf("Platform: macOS\n");
#elif __linux__
    printf("Platform: Linux\n");
#elif __FreeBSD__
    printf("Platform: FreeBSD\n");
#else
    printf("Platform: unknown\n");
#endif
    return 0;
}

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <libgen.h>

#define FALLBACK_VERSION "v0.28.71"

int main(int argc, char *argv[]) {
    // 1. Ultra-fast path for version check (< 2ms)
    if (argc > 1 && (strcmp(argv[1], "-v") == 0 || strcmp(argv[1], "--version") == 0)) {
        char exe_path[1024];
        ssize_t len = readlink("/proc/self/exe", exe_path, sizeof(exe_path) - 1);
        if (len != -1) {
            exe_path[len] = '\0';
            char *dir = dirname(exe_path);
            char ver_path[1024];
            snprintf(ver_path, sizeof(ver_path), "%s/version.txt", dir);
            FILE *f = fopen(ver_path, "r");
            if (f) {
                char buf[64];
                if (fgets(buf, sizeof(buf), f)) {
                    buf[strcspn(buf, "\r\n")] = 0;
                    if (buf[0] == 'v') {
                        puts(buf);
                    } else {
                        printf("v%s\n", buf);
                    }
                    fclose(f);
                    return 0;
                }
                fclose(f);
            }
        }
        puts(FALLBACK_VERSION);
        return 0;
    }

    // 2. Delegate execution to actx-core
    char exe_path[1024];
    ssize_t len = readlink("/proc/self/exe", exe_path, sizeof(exe_path) - 1);
    if (len != -1) {
        exe_path[len] = '\0';
        char *dir = dirname(exe_path);
        char core_path[1024];
        snprintf(core_path, sizeof(core_path), "%s/actx-core", dir);
        argv[0] = core_path;
        execv(core_path, argv);
    }

    execvp("actx-core", argv);
    perror("execvp actx-core");
    return 1;
}

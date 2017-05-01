#include <windows.h>
#include <iostream>

static HANDLE stdIn;
static DWORD ORIG_MODE;

void disableRawMode()
{
    std::cout << "gonna to disable raw mode...";
    SetConsoleMode(stdIn, ORIG_MODE);
    std::cout << "done\n";
}

int main(int argc, char* argv[])
{
    stdIn = GetStdHandle(STD_INPUT_HANDLE);

    std::cout << "gonna get current console mode\n";
    if (!GetConsoleMode(stdIn, &ORIG_MODE)) {
        return 1;
    }

    atexit(disableRawMode);

    std::cout << "gonna set non-echo and non line input\n";
    DWORD rawMode = ORIG_MODE;
    rawMode &= ~(ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT);
    if (!SetConsoleMode(stdIn, rawMode)) {
        return 2;
    }

    std::cout << "time to read some chars\n";

    char buf[] = {0};
    DWORD num = 0;
    bool running = true;
    while (running) {
        if (!ReadConsole(stdIn, &buf, 1, &num, NULL)) {
            return 3;
        }

        printf("read a char: %c\n", buf[0]);

        if (buf[0] == 'q') {
            running = false;
        }
    }
}

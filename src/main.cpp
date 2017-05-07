#include <windows.h>
#include <iostream>

static HANDLE stdIn;
static HANDLE stdOut;
static DWORD ORIG_INPUT_MODE;
static DWORD ORIG_OUTPUT_MODE;

#define NUM_EVENTS 1
#define CTRL_KEY(k) ((k) & 0x1f)

void die(const char* s)
{
    perror(s);
    exit(1);
}

void disableRawMode()
{
    SetConsoleMode(stdIn, ORIG_INPUT_MODE);
    SetConsoleMode(stdOut, ORIG_OUTPUT_MODE);
}

int main(int argc, char* argv[])
{
    stdIn = GetStdHandle(STD_INPUT_HANDLE);
    stdOut = GetStdHandle(STD_OUTPUT_HANDLE);

    if (!GetConsoleMode(stdIn, &ORIG_INPUT_MODE)) {
        die("failed to get stdin mode");
    }

    if (!GetConsoleMode(stdOut, &ORIG_OUTPUT_MODE)) {
        die("failed to get stdout mode");
    }

    atexit(disableRawMode);

    {
        DWORD rawMode = ORIG_INPUT_MODE;
        rawMode &= ~(ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT | ENABLE_PROCESSED_INPUT);
        if (!SetConsoleMode(stdIn, rawMode)) {
            die("failed to set stdin mode");
        }
    }

    {
        DWORD enable_virtual_terminal_processing = 4;
        DWORD disable_newline_auto_return = 8;

        DWORD rawMode = ORIG_OUTPUT_MODE;
        rawMode |= (disable_newline_auto_return | enable_virtual_terminal_processing);
        if (!SetConsoleMode(stdOut, rawMode)) {
            die("failed to set stdout mode");
        }
    }

    bool running = true;
    while (running) {
        char buf[NUM_EVENTS] = {0};
        DWORD num = 0;

        DWORD waiting = WaitForSingleObject(stdIn, 1000);
        if (waiting == WAIT_OBJECT_0) {
            INPUT_RECORD input[NUM_EVENTS];
            DWORD numEventsRead;
            if (!ReadConsoleInput(stdIn, input, NUM_EVENTS, &numEventsRead)) {
                die("failed to ReadConsoleInput");
            }

            for (int i = 0; i < NUM_EVENTS; i++) {
                switch (input[i].EventType) {
                case KEY_EVENT: {
                    KEY_EVENT_RECORD record = input[i].Event.KeyEvent;
                    if (record.bKeyDown) {
                        buf[i] = record.uChar.AsciiChar;
                    }
                } break;
                }
            }
        }

        char output[64];
        DWORD numChars = sprintf(output, "read a char: %c\r\n", buf[0]);
        DWORD writtenChars;
        WriteConsole(stdOut, output, numChars, &writtenChars, NULL);

        if (buf[0] == CTRL_KEY('q')) {
            running = false;
        }
    }
}

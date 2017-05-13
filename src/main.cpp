#include <windows.h>
#include <iostream>

typedef struct config {
    DWORD orig_stdin_mode;
    DWORD orig_stdout_mode;
    int screenrows;
    int screencols;
} config;

static HANDLE stdIn;
static HANDLE stdOut;
static config E;

#define NUM_EVENTS 1
#define CTRL_KEY(k) ((k)&0x1f)

void clsConsole() {
    COORD origin = {0, 0};
    DWORD charsWritten = 0;
    CONSOLE_SCREEN_BUFFER_INFO csbi;
    DWORD sizeOfConsole = 0;

    if (!GetConsoleScreenBufferInfo(stdOut, &csbi)) {
        return;
    }

    sizeOfConsole = csbi.dwSize.X * csbi.dwSize.Y;

    if (!FillConsoleOutputCharacter(stdOut, (TCHAR)' ', sizeOfConsole, origin,
                                    &charsWritten)) {
        return;
    }

    if (!GetConsoleScreenBufferInfo(stdOut, &csbi)) {
        return;
    }

    if (!FillConsoleOutputAttribute(stdOut, csbi.wAttributes, sizeOfConsole,
                                    origin, &charsWritten)) {
        return;
    }

    SetConsoleCursorPosition(stdOut, origin);
}

void ansiClearScreen() {
    DWORD writtenChars;
    // FIXME: this likely only works on recent Windows 10
    WriteConsole(stdOut, "\x1b[2J", 4, &writtenChars, NULL);
    WriteConsole(stdOut, "\x1b[H", 3, &writtenChars, NULL);
}

void clearScreen() {
    ansiClearScreen();
    // clsConsole();
}

void drawRows() {
    DWORD writtenChars;
    int numRows = E.screenrows;
    for (int y = 0; y < numRows; y++) {
        WriteConsole(stdOut, "~", 1, &writtenChars, NULL);

        if (y < numRows - 1) {
            WriteConsole(stdOut, "\r\n", 2, &writtenChars, NULL);
        }
        // else {
        //     WriteConsole(stdOut, "-", 1, &writtenChars, NULL);
        // }
    }
}

void win32SetCursorOrigin() {
    COORD origin = {0, 0};
    SetConsoleCursorPosition(stdOut, origin);
}

void ansiSetCursorOrigin() {
    DWORD writtenChars;
    WriteConsole(stdOut, "\x1b[H", 3, &writtenChars, NULL);
}

void gotoOrigin() {
    // win32SetCursorOrigin();
    ansiSetCursorOrigin();
}

void refreshScreen() {
    clearScreen();

    drawRows();

    gotoOrigin();
}

void die(const char* s) {
    clearScreen();

    perror(s);
    exit(1);
}

void disableRawMode() {
    SetConsoleMode(stdIn, E.orig_stdin_mode);
    SetConsoleMode(stdOut, E.orig_stdout_mode);
}

void enableRawMode() {
    stdIn = GetStdHandle(STD_INPUT_HANDLE);
    stdOut = GetStdHandle(STD_OUTPUT_HANDLE);

    if (!GetConsoleMode(stdIn, &E.orig_stdin_mode)) {
        die("failed to get stdin mode");
    }

    if (!GetConsoleMode(stdOut, &E.orig_stdout_mode)) {
        die("failed to get stdout mode");
    }

    atexit(disableRawMode);

    {
        DWORD rawMode = E.orig_stdin_mode;
        rawMode &=
            ~(ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT | ENABLE_PROCESSED_INPUT);
        if (!SetConsoleMode(stdIn, rawMode)) {
            die("failed to set stdin mode");
        }
    }

    {
        DWORD enable_virtual_terminal_processing = 4;
        DWORD disable_newline_auto_return = 8;

        DWORD rawMode = E.orig_stdout_mode;
        rawMode &= ~(ENABLE_WRAP_AT_EOL_OUTPUT);
        rawMode |=
            (disable_newline_auto_return | enable_virtual_terminal_processing);
        if (!SetConsoleMode(stdOut, rawMode)) {
            die("failed to set stdout mode");
        }
    }
}

int getWindowSize(int* rows, int* cols) {
    // TODO: how to initialize to zero values without
    // writing all the fields
    CONSOLE_SCREEN_BUFFER_INFO info = {0};
    if (!GetConsoleScreenBufferInfo(stdOut, &info) || info.dwSize.X == 0) {
        return -1;
    }
    *rows = info.dwSize.X;
    *cols = info.dwSize.Y;
    return 0;
}

char readKey() {
    char character = '\0';

    DWORD waiting = WaitForSingleObject(stdIn, 1000);
    if (waiting == WAIT_OBJECT_0) {
        INPUT_RECORD input[1];
        DWORD numEventsRead;
        if (!ReadConsoleInput(stdIn, input, NUM_EVENTS, &numEventsRead)) {
            die("failed to ReadConsoleInput");
        }

        if (numEventsRead == 1) {
            switch (input[0].EventType) {
                case KEY_EVENT: {
                    KEY_EVENT_RECORD record = input[0].Event.KeyEvent;
                    if (record.bKeyDown) {
                        character = record.uChar.AsciiChar;
                    }
                } break;
            }
        }
    }

    return character;
}

void processKeyPress() {
    char c = readKey();

    if (c == CTRL_KEY('q')) {
        clearScreen();
        exit(0);
    }
}

void initEditor() {
    if (getWindowSize(&E.screenrows, &E.screencols) == -1) {
        die("could not get window size");
    }
}

int main(int argc, char* argv[]) {
    enableRawMode();
    initEditor();

    while (1) {
        refreshScreen();
        processKeyPress();
    }
}

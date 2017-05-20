#include <windows.h>
#include <iostream>

typedef struct config {
    DWORD orig_stdin_mode;
    DWORD orig_stdout_mode;
    int cx, cy;
    int screenrows;
    int screencols;
} config;

static HANDLE stdIn;
static HANDLE stdOut;
static config E;

typedef struct abuf {
    char* b;
    int len;
} abuf;

#define ABUF_INIT \
    { NULL, 0 }

void abAppend(abuf* ab, const char* s, int len) {
    char* newBuffer = (char*)realloc(ab->b, ab->len + len);

    if (newBuffer == NULL) {
        return;
    }
    memcpy(&newBuffer[ab->len], s, len);
    ab->b = newBuffer;
    ab->len += len;
}

void abWrite(abuf* ab) {
    DWORD writtenChars;
    WriteConsole(stdOut, ab->b, ab->len, &writtenChars, NULL);
}

void abFree(abuf* ab) { free(ab->b); }

void abFlush(abuf* ab) {
    abWrite(ab);
    abFree(ab);
}

#define KILO_VERSION "0.0.1"
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

void ansiClearScreen(abuf* ab) {
    abAppend(ab, "\x1b[2J", 4);
    abAppend(ab, "\x1b[H", 3);
}

void clearScreen(abuf* ab) {
    ansiClearScreen(ab);
    // clsConsole();
}

void drawRows(abuf* ab) {
    int numRows = E.screenrows;
    for (int y = 0; y < numRows; y++) {
        if (y == numRows / 3) {
            char welcome[80];
            int welcomelen =
                snprintf(welcome, sizeof(welcome), "Kilo editor -- version %s",
                         KILO_VERSION);
            if (welcomelen > E.screencols) {
                welcomelen = E.screencols;
            }
            int padding = (E.screencols - welcomelen) / 2;
            if (padding) {
                abAppend(ab, "~", 1);
                padding--;
            }
            while (padding--) {
                abAppend(ab, " ", 1);
            }
            abAppend(ab, welcome, welcomelen);
        } else {
            abAppend(ab, "~", 1);
        }

        abAppend(ab, "\x1b[K", 3);
        if (y < numRows - 1) {
            abAppend(ab, "\r\n", 2);
        }
    }
}

void win32SetCursorOrigin() {
    COORD origin = {0, 0};
    SetConsoleCursorPosition(stdOut, origin);
}

void ansiSetCursorOrigin(abuf* ab) { abAppend(ab, "\x1b[H", 3); }

void gotoOrigin(abuf* ab) {
    // win32SetCursorOrigin();
    ansiSetCursorOrigin(ab);
}

void win32ShowHideCursor(bool hide) {
    CONSOLE_CURSOR_INFO info;
    if (!GetConsoleCursorInfo(stdOut, &info)) {
        return;
    }

    info.bVisible = !hide;

    if (!SetConsoleCursorInfo(stdOut, &info)) {
        return;
    }
}

void ansiShowHideCursor(abuf* ab, bool hide) {
    if (hide) {
        abAppend(ab, "\x1b[?25l", 6);
    } else {
        abAppend(ab, "\x1b[?25h", 6);
    }
}

void showHideCursor(abuf* ab, bool hide) {
    ansiShowHideCursor(ab, hide);
    // win32ShowHideCursor(hide);
}

void resetCursor(abuf* ab) {
    char buf[32];
    snprintf(buf, sizeof(buf), "\x1b[%d;%dH", E.cy + 1, E.cx + 1);
    abAppend(ab, buf, strlen(buf));
}

void refreshScreen() {
    abuf ab = ABUF_INIT;

    showHideCursor(&ab, true);
    gotoOrigin(&ab);

    drawRows(&ab);

    resetCursor(&ab);

    showHideCursor(&ab, false);

    abWrite(&ab);

    abFree(&ab);
}

void die(const char* s) {
    abuf ab = ABUF_INIT;
    clearScreen(&ab);
    abWrite(&ab);
    abFree(&ab);

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

int ansiGetWindowSize(int* rows, int* cols) {
    DWORD CharsWritten;
    WriteConsole(stdOut, "\x1b[999C\x1b[999B", 12, &CharsWritten, NULL);
    if (CharsWritten != 12) {
        return -1;
    }

    char buf[32];
    unsigned int i = 0;

    WriteConsole(stdOut, "\x1b[6n", 4, &CharsWritten, NULL);
    if (CharsWritten != 4) {
        return -1;
    }

    while (i < sizeof(buf) - 1) {
        DWORD CharsRead;
        ReadConsole(stdIn, &buf[i], 1, &CharsRead, NULL);
        if (CharsRead != 1) {
            break;
        }
        if (buf[i] == 'R') {
            break;
        }
        ++i;
    }

    buf[i] = '\0';

    if (buf[0] != '\x1b' || buf[1] != '[') {
        return -1;
    }
    if (sscanf(&buf[2], "%d;%d", rows, cols) != 2) {
        return -1;
    }

    return 0;
}

int getWindowSize(int* rows, int* cols) {
    int ansiRows;
    int ansiCols;
    ansiGetWindowSize(&ansiRows, &ansiCols);

    // TODO: how to initialize to zero values without
    // writing all the fields
    CONSOLE_SCREEN_BUFFER_INFO info = {0};
    if (!GetConsoleScreenBufferInfo(stdOut, &info) || info.dwSize.X == 0) {
        return -1;
    }
    *rows = info.srWindow.Bottom - info.srWindow.Top + 1;
    *cols = info.srWindow.Right - info.srWindow.Left + 1;

    if ((*rows != ansiRows) || (*cols != ansiCols)) {
        return 0;
    }
    return 0;
}

int read(HANDLE handle, void* buf, size_t count) {
    char* charBuf = (char*)buf;
    size_t readSoFar = 0;
    INPUT_RECORD* inputs = (INPUT_RECORD*)malloc(sizeof(INPUT_RECORD) * count);
    while (readSoFar < count) {
        DWORD waiting = WaitForSingleObject(handle, 1000);

        if (waiting == WAIT_OBJECT_0) {
            DWORD numEventsRead;
            PINPUT_RECORD input = &inputs[readSoFar];
            if (!ReadConsoleInput(handle, input, 1, &numEventsRead)) {
                return -1;
            }

            if ((numEventsRead == 1) && (input->EventType == KEY_EVENT)) {
                KEY_EVENT_RECORD record = input->Event.KeyEvent;
                if (record.bKeyDown) {
                    CHAR key = '\0';
                    switch (record.wVirtualKeyCode) {
                    case VK_UP: key = 'w'; break;
                    case VK_DOWN: key = 's'; break;
                    case VK_LEFT: key = 'a'; break;
                    case VK_RIGHT: key = 'd'; break;
                    default: key = record.uChar.AsciiChar; break;
                    }
                    charBuf[readSoFar++] = key;
                }
            }
        }
    }
    return readSoFar;
}

char readKey() {
    char character = '\0';

    int bytesRead;
    bytesRead = read(stdIn, &character, 1);
    if (bytesRead == -1) {
        die("failed to read from stdIn");
    }

    return character;
}

void moveCursor(char key) {
    switch (key) {
        case 'a': {
            E.cx--;
        } break;
        case 'd': {
            E.cx++;
        } break;
        case 'w': {
            E.cy--;
        } break;
        case 's': {
            E.cy++;
        } break;
    }
}

void processKeyPress() {
    char c = readKey();

    switch (c) {
        case CTRL_KEY('q'): {
            abuf ab = ABUF_INIT;
            clearScreen(&ab);
            abWrite(&ab);
            abFree(&ab);
            exit(0);
        } break;
        case 'w':
        case 's':
        case 'a':
        case 'd': {
            moveCursor(c);
        } break;
    }
}

void initEditor() {
    E.cx = 0;
    E.cy = 0;

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

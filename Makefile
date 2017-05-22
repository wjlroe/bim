all: kilo win_size

kilo: kilo.c
	$(CC) kilo.c -o kilo -Wall -Wextra -pedantic -std=c99 -g

win_size: win_size.c
	$(CC) win_size.c -o win_size -Wall -Wextra -pedantic -std=c99 -g

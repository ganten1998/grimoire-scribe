#include <stdio.h>
#include <stdlib.h>

#include "font8x8_basic.h"

void usage(char *exec) {
    printf("Usage: %s <char_code>\n", exec);
    printf("       <char_code> Decimal character code between 0 and 127\n");
}

void render(const unsigned char *bitmap) {
    unsigned int set;
    for (unsigned int x=0U; x < 8U; x++) {
        for (unsigned int y=0U; y < 8U; y++) {
            set = bitmap[x] & 1 << y;
            printf("%c", set ? 'X' : ' ');
        }
        printf("\n");
    }
}

int main(int argc, char **argv) {
    int ord;
    if (argc != 2) {
        usage(argv[0]);
        return 1;
    }
    ord = atoi(argv[1]);
    if (ord > 127 || ord < 0) {
        usage(argv[0]);
        return 2;
    }
    const unsigned char *bitmap = font8x8_basic[ord];

    render(bitmap);
    return 0;
}

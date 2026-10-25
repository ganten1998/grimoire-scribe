/* Linked into the Kindle binary (see the Makefile): drops .comment, where the compiler and the
   linker write their names and versions ("Homebrew clang version ...", "Linker: LLD ..."). Two
   builds of zig 0.16.0 (Homebrew's and ziglang.org's) make the same code but name themselves
   differently; without .comment their binaries are the same bytes. Nothing loads .comment. */
SECTIONS { /DISCARD/ : { *(.comment) } } INSERT AFTER .bss;

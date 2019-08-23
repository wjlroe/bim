#!/bin/sh

# TODO: parameterise debug/release build
# TODO: pass extra args straight through rather than baking in

valgrind --tool=callgrind --dump-instr=yes --collect-jumps=yes --simulate-cache=yes target/debug/bim -- testfiles/kilo-unix.c

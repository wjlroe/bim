#!/bin/sh

TARGET_DIR="debug"
BIM_ARGS=$*
if [[ "$1" == "release"  ||  "$1" == "debug" ]]; then
        TARGET_DIR=$1
        BIM_ARGS=${*:2}
fi

valgrind --tool=callgrind --dump-instr=yes --collect-jumps=yes --simulate-cache=yes target/${TARGET_DIR}/bim -- ${BIM_ARGS}

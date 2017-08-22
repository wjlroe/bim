#!/bin/bash

if [[ "$TRAVIS_OS_NAME" == "linux" ]]; then
  export DISPLAY=:99.0
  RUN_TESTS="yes"
fi

if [[ "$RUN_TESTS" == "yes" ]]; then
  bats tests/window_dimensions.bats
fi

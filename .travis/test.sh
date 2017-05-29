#!/bin/bash

if [[ "$TRAVIS_OS_NAME" == "linux" ]]; then
  export DISPLAY=:99.0
  xterm -geometry 80x20+10+2 -e ./kilo --debug
fi

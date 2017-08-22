#!/usr/bin/env bats

@test "correct number of rows" {
  xterm -geometry 80x20+10+2 -e ./kilo --debug
  rows="$(cat .kilo_debug|grep -o -e 'rows: [0-9]\{1,\}'|cut -d' ' -f2)"
  [ $rows = 20 ]
}

@test "correct number of cols" {
  xterm -geometry 80x20+10+2 -e ./kilo --debug
  cols="$(cat .kilo_debug|grep -o -e 'cols: [0-9]\{1,\}'|cut -d' ' -f2)"
  [ $cols = 80 ]
}

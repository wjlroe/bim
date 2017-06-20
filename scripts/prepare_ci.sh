#!/usr/bin/env bash

set -x

RUSTFMT_VERSION="0.9.0"

unset CARGO_HOME

install_rustfmt() {
  cargo install --force --vers "${RUSTFMT_VERSION}" rustfmt
}

get_version() {
  rv=$(rustfmt --version)
  [[ "$rv" =~ ([0-9\.]+) ]] && echo "${BASH_REMATCH[1]}"
}

which rustfmt >/dev/null || install_rustfmt

installed_rustfmt_version=$(get_version)

if [ "x${installed_rustfmt_version}x" != "x${RUSTFMT_VERSION}x" ]; then
  install_rustfmt
fi

installed_rustfmt_version=$(get_version)

if [ "x${installed_rustfmt_version}x" != "x${RUSTFMT_VERSION}x" ]; then
  echo "Wanted rustfmt ${RUSTFMT_VERSION} but we have ${installed_rustfmt_version}!"
  exit 1
fi

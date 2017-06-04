#!/usr/bin/env bash

RUSTFMT_VERSION="0.8.4"

install_rustfmt() {
  cargo install --force --vers "${RUSTFMT_VERSION}" rustfmt
}

which rustfmt >/dev/null || install_rustfmt

installed_rustfmt_version=$(rustfmt  --version | cut -f 1 -d ' ')

if [ "x${installed_rustfmt_version}x" != "x${RUSTFMT_VERSION}x" ]; then
  install_rustfmt
fi

installed_rustfmt_version=$(rustfmt  --version | cut -f 1 -d ' ')

if [ "x${installed_rustfmt_version}x" != "x${RUSTFMT_VERSION}x" ]; then
  echo "Wanted rustfmt ${RUSTFMT_VERSION} but we have ${installed_rustfmt_version}!"
  exit 1
fi

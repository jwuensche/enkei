#!/bin/env sh

set -e

# Default prefix
PREFIX=/usr/bin

if [ "$1" = '--prefix' ]
then
	PREFIX=$2
fi

cargo build --release

install -Dm755 target/release/enkei "${PREFIX}/enkei"
install -Dm755 target/release/enkeictl "${PREFIX}/enkeictl"

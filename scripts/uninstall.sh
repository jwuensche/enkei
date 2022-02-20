#!/bin/env sh

set -e


# Default prefix
PREFIX=/usr/bin

if [ "$1" = '--prefix' ]
then
	PREFIX=$2
else
    echo 'If a non-standard $PREFIX has been used to install please specify it here also.'
fi

rm "${PREFIX}/enkei"
rm "${PREFIX}/enkeictl"


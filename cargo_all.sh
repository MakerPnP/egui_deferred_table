#!/bin/sh


cargo "$@"
pushd demos

for f in *; do
    if [ -d "$f" ]; then
        # $f is a directory
        pushd $f
        cargo "$@"
        popd

    fi
done

popd

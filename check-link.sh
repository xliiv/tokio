#!/usr/bin/env bash
set -x

rm -rf target/doc
git co master
RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc --all-features
linkchecker file:///home/xliiv/workspace/tokio/target/doc/tokio/index.html > links-master #2>&1

rm -rf target/doc

git co 1473-fix-misc-links
RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc --all-features
linkchecker file:///home/xliiv/workspace/tokio/target/doc/tokio/index.html > links-final #2>&1

gvimdiff links-master links-final

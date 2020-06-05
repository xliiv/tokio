#!/usr/bin/env bash

set -ex

rg '(struct|trait|enum|macro|fn|attr|type)\.[a-zA-Z0-9_]*\.html|#method.' \
    -t rust \
    -g '!target' \
    -g '!tokio-test' \
    | cut -d ':' -f2-|tr  -d '[:blank:]'

#!/bin/bash

## TODO: simplify script, remove lionco related code, prepare to CI

DIR="$(dirname $0)"
cd $DIR
VERSION=$(cat ../../package.json | jq -r '.version')
if [[ "$CI" == "true" ]]; then
    VERSION="_$VERSION"
    ORG=neutronorg/lionco-contracts:
else
    VERSION=":$VERSION"
fi
docker build -t ${ORG}aum-messenger-test${VERSION} ./../../../aum_messenger
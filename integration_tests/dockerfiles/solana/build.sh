#!/bin/bash

## TODO: simplify script, remove lionco related code, prepare to CI

DIR="$(dirname $0)"
COMMIT_HASH_OR_BRANCH="v2.3.6"
cd $DIR
VERSION=$(cat ../../package.json | jq -r '.version')
if [[ "$CI" == "true" ]]; then
    VERSION="_$VERSION"
    ORG=structured:
else
    VERSION=":$VERSION"
fi
git clone https://github.com/anza-xyz/agave.git
cd agave
git checkout $COMMIT_HASH_OR_BRANCH
cd .. 

docker buildx build --load --build-context app=. -t ${ORG}solana-test${VERSION} .

rm -rf ./agave

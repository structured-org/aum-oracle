#!/bin/bash

## TODO: simplify script, remove lionco related code, prepare to CI

DIR="$(dirname $0)"
COMMIT_HASH_OR_BRANCH="v8.1.1"
cd $DIR
VERSION=$(cat ../../package.json | jq -r '.version')
if [[ "$CI" == "true" ]]; then
    VERSION="_$VERSION"
    ORG=neutronorg/lionco-contracts:
else
    VERSION=":$VERSION"
fi
git clone https://github.com/neutron-org/neutron
cd neutron
git checkout $COMMIT_HASH_OR_BRANCH
docker buildx build --load --build-context app=. -t ${ORG}neutron-test${VERSION} --build-arg BINARY=neutrond .
cd ..
rm -rf ./neutron
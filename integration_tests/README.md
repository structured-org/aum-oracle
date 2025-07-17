# integration-tests

TODO: write a readme

This repository contains tests for MaxBTC AUM Oracle and respective contracts.

## How to run

### Prerequisites

- node v18.12+
- Docker engine
- yarn

### Prepare

1. run `yarn`
2. run `yarn build-images`

### Run

Execute `yarn test` to run all tests.

Note: if tests fail, run:

```bash
docker-compose -f ./docker-compose-first.yml -p first down --remove-orphans
docker-compose -f ./docker-compose-second.yml -p second down --remove-orphans
docker-compose -f ./docker-compose-satellite.yml -p second down --remove-orphans
```

and then try `yarn test` again.

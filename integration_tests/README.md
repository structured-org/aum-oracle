# integration-tests

This repository contains tests for MaxBTC AUM Oracle and respective contracts.

## How to run

### Prerequisites

- node v18.12+
- Docker engine
- yarn

### Prepare

1. run `yarn`
2. run `make build` in root folder
3. run `cp ./artifacts/* ./integration_tests/artifacts/contracts/`
4. run `yarn build-images`
5. run `yarn test:consensus`

### Run

Execute `yarn test:consensus` to run all tests.

NOTE: `yarn test` is broken for now because of port collision.

Note: if tests fail, run:

```bash
docker-compose -f ./docker-compose-consensus.yml -p first down --remove-orphans
```

and then try `yarn test:consensus` again.

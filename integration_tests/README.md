# integration-tests

This repository contains tests for MaxBTC AUM Oracle and respective contracts.

## How to run

### Prerequisites

- node v18.12+
- Docker engine
- yarn

### Prepare

1. run `yarn`
2. copy all neutron-related contracts into integration_tests/artifacts folder
   - clone https://github.com/neutron-org/neutron-integration-tests repo if didn't have
   - run in neutron-integration-tests `node download_artifacts.js neutron-dao -b main` and `node download_artifacts.js neutron-dev-contracts -b main`
   - run `cp -r ../neutron-integration-tests/contracts ./integration_tests/artifacts`
   - run `cp -r ../neutron-integration-tests/contracts_thirdparty ./integration_tests/artifacts`
3. run `make build` in root folder
4. run `cp ./artifacts/* ./integration_tests/artifacts/contracts/`
5. build slinky oracle contract from [https://github.com/neutron-org/slinky-vault/blob/545118ff29b361cc5b7af6b8fcaf083e5c4bd61c](here) 
   - run `make optimize` in slinky-oracle repo
   - run `cp ../slinky-vault/artifacts/slinky_oracle.wasm ./integration_tests/artifacts/contracts/slinky_oracle.wasm`
6. run `yarn build-images`
7. run `yarn test:consensus`

### Run

Execute `yarn test:consensus` to run all tests.

NOTE: `yarn test` is broken for now because of port collision.

Note: if tests fail, run:

```bash
docker-compose -f ./docker-compose-consensus.yml -p first down --remove-orphans
```

and then try `yarn test:consensus` again.

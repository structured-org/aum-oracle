# Oracle Messenger

Oracle Messenger is a Go daemon that serves as part of the Oracle system. Its responsibilities include retrieving information from other sources (off-chain data such as from Binance, and other-chain data such as Jupiter/Solana for Neutron) and submitting it to the on-chain Oracle Receiver contract.

## How to run

1. Copy `config.yaml.default` as `config.yaml`
2. Populate `config.yaml` with the respective values
3. Run `docker compose up`

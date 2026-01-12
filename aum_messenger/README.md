# Oracle Messenger

Oracle Messenger is a Go daemon that serves as part of the Oracle system. Its responsibilities include retrieving information from other sources (off-chain data such as from Binance, and other-chain data such as Jupiter/Solana for Neutron) and submitting it to the on-chain Oracle Receiver contract.

## How to run

1. Copy `config.yaml.default` as `config.yaml`
2. Populate `config.yaml` with the respective values
3. Run `docker compose up aum_messenger -f ../docker-compose.yml`

## Simulation mode (Docker + REST)

Simulation mode disables submissions and stores the last produced values in memory.

```bash
# From repo root
cp artifacts/aum_messenger/config.yaml.default config.yaml
# Edit config.yaml with your values

docker compose up aum-messenger-sim
```

REST endpoints:
- `GET /health`
- `GET /last` (JSON)

Example:

```bash
curl -s http://127.0.0.1:16400/last | jq .
```

## REST in normal mode

By default the REST server listens on `http://127.0.0.1:16400`.
To override the listen address provide `--rest-laddr`:

```bash
./aum-messenger --config config.yaml --rest-laddr http://127.0.0.1:16400
```

Note (Docker): to access REST from outside the container, bind to `0.0.0.0:16400` and publish the port.

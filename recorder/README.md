# Recorder

Recorder is a Go daemon that serves as part of the Oracle system. It has just one responsibility - periodically execute
'{"record_er": {}}' method in the TWAER contract.

## How to run

1. Copy `config.yaml.default` as `config.yaml`
2. Populate `config.yaml` with the respective values
3. Run `docker compose up recorder -f ../docker-compose.yml`

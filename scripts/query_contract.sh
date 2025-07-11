#!/bin/bash
set -euo pipefail

if [ $# -ne 1 ]; then
  echo "usage: $0 <contract_address>"
  exit 1
fi

ADDR="$1"
NODE=http://localhost:26657

run_query() {
  local label="$1"
  local msg="$2"
  echo ">>> querying $label"
  neutrond q wasm contract-state smart "$ADDR" "$msg" --node "$NODE" --output json | jq .
  echo
}

run_query "get_data"     '{"get_data":{}}'
run_query "get_aum"    '{"get_aum":{}}'
run_query "get_round_info" '{"get_round_info":{}}'

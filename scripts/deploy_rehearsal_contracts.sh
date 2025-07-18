#!/bin/bash
set -euo pipefail

NODE=https://rpc-falcron.pion-1.ntrn.tech
CHAIN_ID=pion-1
FROM=oracle_testnet_owner
KEYRING=test
OWNER=neutron1vc0j05p2h2na0673wusf6vxx0epn7wef72fjw9
GAS_PRICES=0.02untrn

extract_tx_hash() {
  echo "$1" | grep 'txhash:' | awk '{print $2}'
}

get_code_id() {
  neutrond q tx "$1" --node "$NODE" --output json | jq -r '.events[] | select(.type=="store_code") | .attributes[] | select(.key=="code_id") | .value' | head -n1
}

get_contract_addr() {
  neutrond q tx "$1" --node "$NODE" --output json | jq -r '.events[] | select(.type=="instantiate") | .attributes[] | select(.key=="_contract_address") | .value' | head -n1
}

echo ">>> storing jupiter aum"
TX=$(neutrond tx wasm store ./artifacts/jupiter_aum_oracle_contract.wasm --from $FROM --keyring-backend $KEYRING --node $NODE --chain-id $CHAIN_ID --broadcast-mode sync --gas-prices $GAS_PRICES --gas auto --gas-adjustment 1.5 -y)
sleep 5
HASH=$(extract_tx_hash "$TX")
JUPITER_CODE_ID=$(get_code_id "$HASH")
sleep 5

JUPITER_INIT_MSG=$(jq -nc --arg owner "$OWNER" '{
  owner: $owner,
  oracles: ["neutron1muz6g7pj83clpsalkpa3z5t0726mfja2q6535n", "neutron10u3xlmcd3kq4qr876y76cvwn4aw6ts45upwa6v", "neutron18yaxpvq7aqnrywmzzmfa4qel2p94ts7uh6u6cz", "neutron1sfe577afplmt9ntmp3anf5w9f5es4uctyj5pjh", "neutron1xqn0k73myhvc0y9h3u6qs45vek4clp4s8tr7hs"],
  threshold: 3,
  data_delta_ppm: 100000,
  round_length: 15,
  consensus_data_validity_period: 60,
  required_custody_assets: ["SOL", "USDC", "USDT", "WBTC", "WETH"],
  price_data_validity_period: 100
}')
echo ">>> instantiating jupiter aum with msg:"
echo "$JUPITER_INIT_MSG"

TX=$(neutrond tx wasm instantiate $JUPITER_CODE_ID "$JUPITER_INIT_MSG" --from $FROM --keyring-backend $KEYRING --node $NODE --chain-id $CHAIN_ID --broadcast-mode sync --gas-prices $GAS_PRICES --gas auto --gas-adjustment 1.5 -y --label jupiter_aum --admin $OWNER)
sleep 5
HASH=$(extract_tx_hash "$TX")
JUPITER_CONTRACT_ADDR=$(get_contract_addr "$HASH" | tr -d '\n')
sleep 5

echo ">>> storing binance aum + slinky oracle"
TX1=$(neutrond tx wasm store ./artifacts/binance_aum_oracle_contract.wasm --from $FROM --keyring-backend $KEYRING --node $NODE --chain-id $CHAIN_ID --broadcast-mode sync --gas-prices $GAS_PRICES --gas auto --gas-adjustment 1.5 -y)
sleep 5
TX2=$(neutrond tx wasm store ./artifacts/slinky_oracle.wasm --from $FROM --keyring-backend $KEYRING --node $NODE --chain-id $CHAIN_ID --broadcast-mode sync --gas-prices $GAS_PRICES --gas auto --gas-adjustment 1.5 -y)
sleep 5

HASH1=$(extract_tx_hash "$TX1")
HASH2=$(extract_tx_hash "$TX2")
BINANCE_CODE_ID=$(get_code_id "$HASH1")
sleep 5
SLINKY_CODE_ID=$(get_code_id "$HASH2")
sleep 5

echo ">>> instantiating slinky oracle with msg: {}"
TX=$(neutrond tx wasm instantiate $SLINKY_CODE_ID '{}' --from $FROM --keyring-backend $KEYRING --node $NODE --chain-id $CHAIN_ID --broadcast-mode sync --gas-prices $GAS_PRICES --gas auto --gas-adjustment 1.5 -y --label slinky_oracle --admin $OWNER)
sleep 5
HASH=$(extract_tx_hash "$TX")
SLINKY_CONTRACT_ADDR=$(get_contract_addr "$HASH" | tr -d '\n')
sleep 5

BINANCE_INIT_MSG=$(jq -nc --arg owner "$OWNER" \
  --arg addr "$SLINKY_CONTRACT_ADDR" \
  '{
    owner: $owner,
    oracles: ["neutron1muz6g7pj83clpsalkpa3z5t0726mfja2q6535n", "neutron10u3xlmcd3kq4qr876y76cvwn4aw6ts45upwa6v", "neutron18yaxpvq7aqnrywmzzmfa4qel2p94ts7uh6u6cz", "neutron1sfe577afplmt9ntmp3anf5w9f5es4uctyj5pjh", "neutron1xqn0k73myhvc0y9h3u6qs45vek4clp4s8tr7hs"],
    threshold: 3,
    data_delta_ppm: 100000,
    round_length: 15,
    consensus_data_valid_period: 60,
    price_data_valid_period: 100,
    required_binance_positions: ["BTCUSDT", "ETHUSDT", "SOLUSDT"],
    required_binance_spot_assets: ["USDT", "BTC", "ETH", "SOL"],
    price_oracle_contract: $addr
  }')

echo ">>> instantiating binance aum with msg:"
echo "$BINANCE_INIT_MSG"
echo ">>> using slinky oracle addr: [$SLINKY_CONTRACT_ADDR]"

TX=$(neutrond tx wasm instantiate $BINANCE_CODE_ID "$BINANCE_INIT_MSG" \
  --from $FROM --keyring-backend $KEYRING --node $NODE --chain-id $CHAIN_ID \
  --broadcast-mode sync --gas-prices $GAS_PRICES --gas auto --gas-adjustment 1.5 \
  -y --label binance_aum --admin $OWNER)
HASH=$(extract_tx_hash "$TX")
sleep 5
BINANCE_CONTRACT_ADDR=$(get_contract_addr "$HASH" | tr -d '\n')

echo
echo "==== summary ===="
echo "jupiter aum contract:  $JUPITER_CONTRACT_ADDR"
echo "binance aum contract:  $BINANCE_CONTRACT_ADDR"

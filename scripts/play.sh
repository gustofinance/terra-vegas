#!/bin/bash

DIFF=10
R=$$
while true
do
  BET_NUMBER=$(($(($RANDOM%$DIFF))+3))
  echo betting $BET_NUMBER
  echo $WALLET_PASSWORD | terrad tx wasm execute $GAME_CONTRACT '{"bet": {"outcome": '$BET_NUMBER'}}' 10000000uusd   --from player-2 --chain-id $CHAIN_ID   --gas=auto --fees=500000uusd --broadcast-mode=block --node=$NODE_URL -y
  sleep 15
  if terrad query wasm  contract-store $GAME_CONTRACT '{"player_rewards": {"addr":"'"$PLAYER_2_ADDR"'"}}' --node=$NODE_URL | grep rewards |grep -v "\"0\"" ; then
  echo "matched"
  echo receiving rewards
  echo $WALLET_PASSWORD | terrad tx wasm execute $GAME_CONTRACT '{"receive_rewards":{}}'    --from player-2 --chain-id $CHAIN_ID  --gas-adjustment 1.4  --gas=auto --fees=1000000uusd --broadcast-mode=block --node=$NODE_URL -y
  fi
done

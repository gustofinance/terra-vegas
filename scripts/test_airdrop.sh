function deploy_airdrop {
  echo "Deploying merkle-airdrop contract"

  init_msg='{"cw20_token_address":"%s"}\n'
  # token address passed as an argument
  cw20_token_address=$1

  msg=$(printf "$init_msg" "$cw20_token_address")

  printf "%s\n" "12345678" | ./terrad tx wasm instantiate 4 $msg --yes --from $test_account --chain-id=$chain_id --fees=10000uluna --gas=auto --broadcast-mode=block --node $node

}

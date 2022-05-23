function deploy_token {
  echo "Deploying token contract"

  init_msg='{"name":"%s","symbol":"%s","decimals":%s,"initial_balances":%s}\n'

  name=test_token
  symbol=TTTT
  decimals=6

  balances='[{"address":"%s","amount":"%s"},{"address":"%s","amount":"%s"}]'
  # ico address passed as an argument
  ico_addr=terra1fmv5elkaknrkp5czftvxxma03c26mc72zylwy5
  ico_amount=5400000
  # some other address
  addr2=terra1dcegyrekltswvyy0xy69ydgxn9x8x32zdtapd8
  amount2=100000

  initial_balances=$(printf "$balances" "$ico_addr" "$ico_amount" "$addr2" "$amount2")
  
  msg=$(printf "$init_msg" "$name" "$symbol" "$decimals" "$initial_balances")

  terrad tx wasm instantiate $token_code_id $msg --yes --from $test_account --chain-id=$chain_id --fees=10000uluna --gas=auto --broadcast-mode=block --node $node

}

function exec_token {
  case $1 in 
    balance)
      token_balance
      ;;
    send)
      send
      ;;
    *)
      echo "unknow input"
      ;;
  esac
}

function token_balance {
  exec_msg='{"balance":{"address":"%s"}}\n'

  msg=$(printf "$exec_msg" "$ico_contract")

  terrad query wasm contract-store $token_contract $msg --node $node

}

# {"stake_voting_tokens":{}}
# eyJzdGFrZV92b3RpbmdfdG9rZW5zIjp7fX0= in base64

function send {
  exec_msg='{"send":{"contract":"%s", "amount": "%s", "msg":"%s"}}\n'

  # {"stake_voting_tokens":{}}
  # eyJzdGFrZV92b3RpbmdfdG9rZW5zIjp7fX0= in base64
  send_msg='eyJzdGFrZV92b3RpbmdfdG9rZW5zIjp7fX0='

  msg=$(printf "$exec_msg" "$governance_contract" "100" "$send_msg")

  echo $msg > /dev/tty

  terrad tx wasm execute $token_contract "$msg" --yes --from $test_account --chain-id=$chain_id --fees=1000000uluna --gas-adjustment 1.4 --gas=auto --broadcast-mode=block --node $node

}

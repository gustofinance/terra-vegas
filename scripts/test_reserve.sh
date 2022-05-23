function deploy_reserve {
  echo "Deploying reserve contract"

  init_msg='{"anchor_market_address":"%s","anchor_token_address":"%s","gov_contract_address":"%s","threshold":"%s","native_denom":"%s","anchor_denom":"%s"}\n'

  # money market and aUST contracts fron Anchor on testnet bombay-12
  anchor_market_address=terra15dwd5mj8v59wpj0wvt233mf5efdff808c5tkal
  anchor_token_address=terra1ajt556dpzvjwl0kl5tzku3fc3p3knkg9mkv8jl
  governance_contract_addr=$governance_contract
  threshold=1000
  native_denom=uusd
  anchor_denom=aUST

  msg=$(printf "$init_msg" "$anchor_market_address" "$anchor_token_address" "$governance_contract_addr" "$threshold" "$native_denom" "$anchor_denom")

  terrad tx wasm instantiate $reserve_code_id $msg --yes --from $test_account --chain-id=$chain_id --fees=10000uluna --gas=auto --broadcast-mode=block --node $node

}

function exec_reserve {
  case $1 in 
    add_game)
      reserve_add_game
      ;;
    deposit)
      reserve_deposit
      ;;
    request) 
      reserve_request
      ;;
    current_balance) 
      current_balance
      ;;
    bank_send)
      bank_send
      ;;
    *)
      echo "unknow input"
      ;;
  esac
}

function reserve_add_game {
  echo ${FUNCNAME[0]}
  exec_msg='{"add_game":{"addr":"%s"}}\n'

  msg=$(printf "$exec_msg" "$distribution_contract")

  terrad tx wasm execute $reserve_contract $msg --yes --from $test_account --chain-id=$chain_id --fees=1000000uluna --gas-adjustment 1.4 --gas=auto --broadcast-mode=block --node $node

  # echo ./terrad tx wasm execute $contract_addr $msg 1000000uusd --yes --from $test_account --chain-id=$chain_id --fees=1000000uluna --gas-adjustment 1.4 --gas=auto --broadcast-mode=block
}

function reserve_deposit {
  echo ${FUNCNAME[0]}
  exec_msg='{"deposit_funds":{}}\n'

  msg=$(printf "$exec_msg")

  terrad tx wasm execute $reserve_contract $msg 1000uusd --yes --from $test_account --chain-id=$chain_id --fees=1000000uluna --gas-adjustment 1.4 --gas=auto --broadcast-mode=block --node $node

  # echo ./terrad tx wasm execute $contract_addr $msg 1000000uusd --yes --from $test_account --chain-id=$chain_id --fees=1000000uluna --gas-adjustment 1.4 --gas=auto --broadcast-mode=block
}

function reserve_request {
  echo ${FUNCNAME[0]}
  exec_msg='{"request_funds":{"amount":"1000000"}}\n'

  msg=$(printf "$exec_msg")

  terrad tx wasm execute $reserve_contract $msg --yes --from $test_account --chain-id=$chain_id --fees=1000000uluna --gas-adjustment 1.4 --gas=auto --broadcast-mode=block --node $node

  # echo ./terrad tx wasm execute $contract_addr $msg 1000000uusd --yes --from $test_account --chain-id=$chain_id --fees=1000000uluna --gas-adjustment 1.4 --gas=auto --broadcast-mode=block
}

function current_balance {
  exec_msg='{"current_balance":{}}\n'

  msg=$(printf "$exec_msg" "$ico_contract")

  terrad query wasm contract-store $reserve_contract $msg --node $node
}

function bank_send {
  terrad tx bank send test1 $reserve_contract 5000000uusd --gas=auto --fees=1000000uluna -y --chain-id localterra
}
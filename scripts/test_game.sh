function deploy_game {
  echo "Deploying game contract"

  init_msg='{"native_denom":"%s","advantage_value":"%s","max_number_of_bets":%s,"round_duration":%s,"max_cashflow":"%s","terrand_address":"%s","reserve_address":"%s"}\n'

  native_denom=uusd
  advantage_value='0.01'
  max_number_of_bets=1
  round_duration=1000000
  max_cashflow=1000
  # terrand on testnet bombay-12
  terrand_address=terra1a62jxn3hh54fa5slan4dkd7u6v4nzgz3pjhygm
  # reserve contract address passed as an argument
  reserve_address=$1

  msg=$(printf "$init_msg" "$native_denom" "$advantage_value" "$max_number_of_bets" "$round_duration" "$max_cashflow" "$terrand_address" "$reserve_address")

  printf "%s\n" "12345678" | ./terrad tx wasm instantiate 2 $msg --yes --from $test_account --chain-id=$chain_id --fees=10000uluna --gas=auto --broadcast-mode=block --node $node
  # echo ./terrad tx wasm instantiate 2 $msg --yes --from $test_account --chain-id=$chain_id --fees=10000uluna --gas=auto --broadcast-mode=block --node $node
}

function exec_game {
  case $1 in 
    bet)
      game_bet
      ;;
    end_round)
      game_end_round
      ;;
    receive_tokens) 
      game_receive_rewards
      ;;
    *)
      echo "unknow input"
      ;;
  esac
}

function game_bet {
  echo ${FUNCNAME[0]}
  exec_msg='{"bet":{"outcome":%s}}\n'
  outcome=4

  msg=$(printf "$exec_msg" "$outcome")

  printf "%s\n" "12345678" | ./terrad tx wasm execute $game_contract $msg 1000000uusd --yes --from $test_account --chain-id=$chain_id --fees=1000000uluna --gas-adjustment 1.4 --gas=auto --broadcast-mode=block --node $node

  # echo ./terrad tx wasm execute $contract_addr $msg 1000000uusd --yes --from $test_account --chain-id=$chain_id --fees=1000000uluna --gas-adjustment 1.4 --gas=auto --broadcast-mode=block
}

function game_end_round {
  echo ${FUNCNAME[0]}
  exec_msg='{"end_round":{}}\n'

  msg=$(printf "$exec_msg")

  printf "%s\n" "12345678" | ./terrad tx wasm execute $game_contract $msg --yes --from $test_account --chain-id=$chain_id --fees=1000000uluna --gas=auto --broadcast-mode=block --node $node

  # echo ./terrad tx wasm execute $contract_addr $msg --yes --from $test_account --chain-id=$chain_id --fees=1000000uluna --gas=auto --broadcast-mode=block
}

function game_receive_rewards {
  echo ${FUNCNAME[0]}
  exec_msg='{"reseive_revards":{}}\n'

  msg=$(printf "$exec_msg")

  printf "%s\n" "12345678" | ./terrad tx wasm execute $game_contract $msg --yes --from $test_account --chain-id=$chain_id --fees=1000000uluna --gas=auto --broadcast-mode=block --node $node

  # echo ./terrad tx wasm execute $contract_addr $msg --yes --from $test_account --chain-id=$chain_id --fees=1000000uluna --gas=auto --broadcast-mode=block
}

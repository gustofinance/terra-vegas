function deploy_ico {
  echo "Deploying ico contract"

  init_msg='{"privatesale_allocation":"%s","privatesale_price":"%s","privatesale_duration":%s,"publicsale_allocation":"%s","publicsale_initial_price":"%s","publicsale_final_price":"%s","publicsale_duration":%s,"price_denom":"%s","final_distribution_percentages":%s}\n'

  privatesale_allocation=600000
  privatesale_price=1000000
  privatesale_duration=10
  publicsale_allocation=5400000
  publicsale_initial_price=1000000
  publicsale_final_price=5000000
  publicsale_duration=10000000
  price_denom=uusd

  distribution='[{"addr":"%s","percentage":"%s"},{"addr":"%s","percentage":"%s"}]'
  # all percentages are set up here
  # reserve contract address passed as an argument
  reserve_addr=$1
  reserve_percentage=0.5
  # some other address
  addr2=terra1dcegyrekltswvyy0xy69ydgxn9x8x32zdtapd8
  percentage2=0.5

  final_distribution_percentages=$(printf "$distribution" "$reserve_addr" "$reserve_percentage" "$addr2" "$percentage2")
  
  msg=$(printf "$init_msg" "$privatesale_allocation" "$privatesale_price" "$privatesale_duration" "$publicsale_allocation" "$publicsale_initial_price" "$publicsale_final_price" "$publicsale_duration" "$price_denom" "$final_distribution_percentages")

  printf "%s\n" "12345678" | ./terrad tx wasm instantiate 3 $msg --yes --from $test_account --chain-id=$chain_id --fees=10000uluna --gas=auto --broadcast-mode=block --node $node
}

function exec_ico {
  case $1 in 
    whitelist)
      ico_whitelist
      ;;
    start)
      ico_start
      ;;
    set_token_addr)
      ico_set_token_addr
      ;;
    buy) 
      ico_buy
      ;;
    end)
      ico_end
      ;;
    receive_tokens) 
      ico_receive_tokens
      ;;
    query_balance) 
      ico_query_balance
      ;;
    query_prices)  
      ico_query_prices
      ;;
    query_state) 
      ico_query_state
      ;;
    *)
      echo "unknow input"
      ;;
  esac
}

function ico_whitelist {
  echo ${FUNCNAME[0]}
  exec_msg='{"add_to_whitelist":{"addr":"%s"}}\n'

  msg=$(printf "$exec_msg" "$self_addr")

  printf "%s\n" "12345678" | ./terrad tx wasm execute $ico_contract $msg --yes --from $test_account --chain-id=$chain_id --fees=1000000uluna --gas=auto --broadcast-mode=block --node $node
  # echo ./terrad tx wasm execute $contract_addr $msg --yes --from $test_account --chain-id=$chain_id --fees=1000000uluna --gas=auto --broadcast-mode=block
}

function ico_start {
  echo ${FUNCNAME[0]}
  exec_msg='{"start_i_c_o":{}}\n'

  msg=$(printf "$exec_msg")

  printf "%s\n" "12345678" | ./terrad tx wasm execute $ico_contract $msg --yes --from $test_account --chain-id=$chain_id --fees=1000000uluna --gas=auto --broadcast-mode=block --node $node
  # echo ./terrad tx wasm execute $contract_addr $msg --yes --from $test_account --chain-id=$chain_id --fees=1000000uluna --gas=auto --broadcast-mode=block
}

function ico_end {
  echo ${FUNCNAME[0]}
  exec_msg='{"end_i_c_o":{}}\n'

  msg=$(printf "$exec_msg")

  printf "%s\n" "12345678" | ./terrad tx wasm execute $ico_contract $msg --yes --from $test_account --chain-id=$chain_id --fees=1000000uluna --gas=auto --broadcast-mode=block --node $node
  # echo ./terrad tx wasm execute $contract_addr $msg --yes --from $test_account --chain-id=$chain_id --fees=1000000uluna --gas=auto --broadcast-mode=block
}

function ico_set_token_addr {
  echo ${FUNCNAME[0]}
  exec_msg='{"set_token_address":{"addr":"%s"}}\n'

  msg=$(printf "$exec_msg" "$token_contract")

  printf "%s\n" "12345678" | ./terrad tx wasm execute $ico_contract $msg --yes --from $test_account --chain-id=$chain_id --fees=1000000uluna --gas=auto --broadcast-mode=block --node $node
  # echo ./terrad tx wasm execute $contract_addr $msg --yes --from $test_account --chain-id=$chain_id --fees=1000000uluna --gas=auto --broadcast-mode=block
}

function ico_buy {
  echo ${FUNCNAME[0]}
  exec_msg='{"buy":{"amount":"%s"}}\n'
  # amount=5399900
  # price=16199899796300uusd
  # amount=100
  amount=5400000
  # price=100003700uusd 

  msg=$(printf "$exec_msg" "$amount")

  printf "%s\n" "12345678" | ./terrad tx wasm execute $ico_contract $msg 16200000000000uusd --yes --from $test_account --chain-id=$chain_id --fees=100000000uusd  --gas-adjustment 1.4 --gas=auto --broadcast-mode=block --node $node
  # echo ./terrad tx wasm execute $contract_addr $msg --yes --from $test_account --chain-id=$chain_id --fees=1000000uluna --gas=auto --broadcast-mode=block
}

function ico_receive_tokens {
  echo ${FUNCNAME[0]}
  exec_msg='{"receive_tokens":{}}\n'

  msg=$(printf "$exec_msg")

  printf "%s\n" "12345678" | ./terrad tx wasm execute $ico_contract $msg --yes --from $test_account --chain-id=$chain_id --fees=1000000uluna --gas-adjustment 1.4 --gas=auto --broadcast-mode=block --node $node
  # echo ./terrad tx wasm execute $contract_addr $msg --yes --from $test_account --chain-id=$chain_id --fees=1000000uluna --gas=auto --broadcast-mode=block
}

function ico_query_balance {
  echo ${FUNCNAME[0]}
  exec_msg='{"balance":{"addr":"%s"}}\n'

  msg=$(printf "$exec_msg" "$self_addr")

  echo "User balance"
  ./terrad query wasm contract-store $ico_contract $msg --node $node
  echo "Contract balance"
  ./terrad  query bank balances $ico_contract
  # echo ./terrad query wasm contract-store $ico_contract $msg
}

function ico_query_prices {
  echo ${FUNCNAME[0]}
  exec_msg='{"prices_for_amount":{"amount":"5400000"}}\n'
  # a=5399900

  msg=$(printf "$exec_msg")

  ./terrad query wasm contract-store $ico_contract $msg --node $node

  # echo ./terrad tx wasm execute $contract_addr $msg --yes --from $test_account --chain-id=$chain_id --fees=1000000uluna --gas=auto --broadcast-mode=block
}
function ico_query_state {
  echo ${FUNCNAME[0]}
  exec_msg='{"i_c_o_state":{}}\n'

  msg=$(printf "$exec_msg")

  ./terrad query wasm contract-store $ico_contract $msg --node $node

  # echo ./terrad tx wasm execute $contract_addr $msg --yes --from $test_account --chain-id=$chain_id --fees=1000000uluna --gas=auto --broadcast-mode=block
}

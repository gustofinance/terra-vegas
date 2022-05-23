function deploy_distribution {
  echo "Deploying distribution contract"

  init_msg='{"reserve_contract_addr":"%s","governance_token_addr":"%s","reward_denom":"%s","unbonding_period":%s,"distribution_ratio":%s}\n'

  governance_contract_addr=$governance_contract
  reserve_contract_addr=$reserve_contract
  reward_denom=uusd
  unbonding_period=0
  distribution_ratio=2
  
  msg=$(printf "$init_msg" "$reserve_contract_addr" "$governance_contract_addr" "$reward_denom" "$unbonding_period" "$distribution_ratio")

  terrad tx wasm instantiate $distribution_code_id $msg --yes --from $test_account --chain-id=$chain_id --fees=10000uluna --gas=auto --broadcast-mode=block --node $node
}

function exec_distribution {
  case $1 in 
    holder)
      holder $2
      ;;
    receive)
      receive
      ;;
    update_global_index)
      update_global_index
      ;;
    claim)
      claim
      ;;
    update_reserve)
      update_reserve $2
      ;;
    *)
      echo "unknonw input"
      ;;
  esac
}

function holder {
  exec_msg='{"holder":{"address":"%s"}}\n'

  msg=$(printf "$exec_msg" "$1")

  echo $distribution_contract $msg $1 > /dev/tty

  terrad query wasm contract-store $distribution_contract $msg --node $node

}


function receive {
  exec_msg='{"receive":{"msg":"eyJzdGFrZV92b3RpbmdfdG9rZW5zIjoge319", "sender": "terra1dcegyrekltswvyy0xy69ydgxn9x8x32zdtapd8", "amount": "100"}}\n'

  msg=$(printf "$exec_msg" "$token_contract")
  
  terrad tx wasm execute $distribution_contract "$msg" --yes --from $test_account --chain-id=$chain_id --fees=10000uluna --gas=auto --broadcast-mode=block --node $node
}

function update_global_index {
  exec_msg='{"update_global_index":{}}\n'

  msg=$(printf "$exec_msg")
  
  terrad tx wasm execute $distribution_contract "$msg" --yes --from $test_account --chain-id=$chain_id --fees=10000uluna --gas=auto --broadcast-mode=block --node $node
}

function claim {
  exec_msg='{"claim_rewards":{}}\n'

  msg=$(printf "$exec_msg")
  
  terrad tx wasm execute $distribution_contract "$msg" --yes --from $test_account --chain-id=$chain_id --fees=10000uluna --gas=auto --broadcast-mode=block --node $node
}

function update_reserve {
  exec_msg='{"update_reserve":{"amount":"%s"}}\n'

  msg=$(printf "$exec_msg" "$1")
  
  terrad tx wasm execute $distribution_contract "$msg" --yes --from $test_account --chain-id=$chain_id --fees=10000uluna --gas=auto --broadcast-mode=block --node $node
}
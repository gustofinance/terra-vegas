function deploy_governance {
  echo "Deploying governance contract"

  init_msg='{"quorum":"%s","threshold":"%s","voting_period":%s,"timelock_period":%s,"proposal_deposit":"%s","snapshot_period":%s}\n'

  quorum=0
  threshold=0
  voting_period=0
  timelock_period=0
  proposal_deposit=0
  snapshot_period=0
  
  msg=$(printf "$init_msg" "$quorum" "$threshold" "$voting_period" "$timelock_period" "$proposal_deposit" "$snapshot_period")

  echo $msg > /dev/tty

  terrad tx wasm instantiate $governance_code_id $msg --yes --from $test_account --chain-id=$chain_id --fees=10000uluna --gas=auto --broadcast-mode=block --node $node

}

function exec_governance {
  case $1 in 
    register_contract)
      register_contract
      ;;
    register_distribution)
      register_distribution
      ;;
    config)
      config
      ;;
    *)
      echo "unknow input"
      ;;
  esac
}

function config {
  exec_msg='{"config":{}}\n'

  msg=$(printf "$exec_msg" "$ico_contract")

  terrad query wasm contract-store $governance_contract $msg --node $node
}

function register_contract {
  exec_msg='{"register_contracts":{"token_contract":"%s"}}\n'

  msg=$(printf "$exec_msg" "$token_contract")
  
  terrad tx wasm execute $governance_contract $msg --yes --from $test_account --chain-id=$chain_id --fees=10000uluna --gas=auto --broadcast-mode=block --node $node
}

function register_distribution {
  exec_msg='{"register_distribution_contracts":{"distribution_contract":"%s"}}\n'

  msg=$(printf "$exec_msg" "$distribution_contract")
  
  terrad tx wasm execute $governance_contract $msg --yes --from $test_account --chain-id=$chain_id --fees=10000uluna --gas=auto --broadcast-mode=block --node $node
}

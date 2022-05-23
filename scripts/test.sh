. $(dirname "$0")/test_reserve.sh
. $(dirname "$0")/test_game.sh
. $(dirname "$0")/test_ico.sh
. $(dirname "$0")/test_token.sh
. $(dirname "$0")/test_airdrop.sh
. $(dirname "$0")/test_governance.sh
. $(dirname "$0")/test_distribution.sh

contracts=("reserve-contract" "game-contract" "ico-contract" "merkle-airdrop" "vegas-token-contract")
test_account=test1
chain_id=localterra

remote_node='http://public-node.terra.dev:26657/'
localnode='tcp://localhost:26657'
node=$localnode

local_addr=terra1dcegyrekltswvyy0xy69ydgxn9x8x32zdtapd8
self_addr=$local_addr


function compile {
  local contract=$1
  curr_path=$PWD
  echo "Compiling" $contract
  cd ../$contract
  RUSTFLAGS='-C link-arg=-s' cargo wasm
  mkdir -p ../testing/
  cp ./target/wasm32-unknown-unknown/release/${contract//-/_}.wasm ../testing/
  cd $curr_path
  echo "$contract compiled"
}

function upload {
  local contract=$1
  echo "Uploading" $contract
  printf "%s\n" "12345678" | ./terrad tx wasm store ${contract//-/_}.wasm --yes --from $test_account --chain-id=$chain_id --gas=auto --fees=100000uluna --broadcast-mode=block --node $node
  echo "Uploading $contract finished"
}

function get_addr {
  echo $1 | awk '{ sub(/.*contract_address value: /, ""); sub(/ type:.*/, ""); print }'
}

function deploy {
  local deploy_out=$($@)
  contract=$(get_addr "$deploy_out")
  echo $contract
}

function save_contract {
  echo ${FUNCNAME[0]}
  echo $2 > $1.txt
}

function read_contracts {
  echo ${FUNCNAME[0]}
  read reserve_contract  <<< "$(cat reserve_contract.txt)"
  read game_contract  <<< "$(cat game_contract.txt)"
  read ico_contract <<< "$(cat ico_contract.txt)"
  read token_contract  <<< "$(cat token_contract.txt)"
  read airdrop_contract  <<< "$(cat airdrop_contract.txt)"
  read governance_contract  <<< "$(cat governance_contract.txt)"
  read distribution_contract  <<< "$(cat distribution_contract.txt)"
}

function read_code_ids {
  echo ${FUNCNAME[0]}
  read token_code_id  <<< "$(cat token_code_id.txt)"
  read governance_code_id  <<< "$(cat governance_code_id.txt)"
  read distribution_code_id <<< "$(cat distribution_code_id.txt)"
  read reserve_code_id  <<< "$(cat reserve_code_id.txt)"
}

function print_contracts {
  echo ${FUNCNAME[0]}
  echo $reserve_contract
  echo $game_contract
  echo $ico_contract
  echo $token_contract
  echo $airdrop_contract
}

function balance {
  echo ${FUNCNAME[0]}
  ./terrad  query bank balances $1
}

function test_ico {
    exec_ico set_token_addr
    exec_ico whilelist 
    exec_ico start
    exec_ico buy

    exec_ico query_balance
    exec_ico query_prices
    balance $ico_contract
}

function test_reserve {
    exec_reserve add_game
    balance $reserve_contract
    balance $self_addr
    exec_reserve deposit
    balance $reserve_contract
    balance $self_addr
    exec_reserve request
    balance $reserve_contract
    balance $self_addr
}

case $1 in 
  compile)
    compile "reserve-contract"
    compile "game-contract"
    compile "ico-contract"
    compile "merkle-airdrop"
    compile "vegas-token-contract"
    ;;
  upload)
    case $2 in 
      reserve)
        upload "reserve-contract"
        ;;
      game)
        upload "game-contract"
        ;;
      ico)
        upload "ico-contract"
        ;;
      airdrop)
        upload "merkle-airdrop"
        ;;
      token)
        upload "vegas-token-contract"
        ;;
      *)
        upload "reserve-contract"
        upload "game-contract"
        upload "ico-contract"
        upload "merkle-airdrop"
        upload "vegas-token-contract"
        ;;
    esac
    ;;
  deploy) 
    read_contracts
    read_code_ids
    case $2 in 
      reserve)
        reserve_contract=$(deploy deploy_reserve)
        save_contract reserve_contract $reserve_contract
        ;;
      game)
        game_contract=$(deploy deploy_game $reserve_contract)
        save_contract game_contract $game_contract
        ;;
      ico)
        ico_contract=$(deploy deploy_ico $reserve_contract)
        save_contract ico_contract $ico_contract
        ;;
      token)
        token_contract=$(deploy deploy_token $ico_contract)
        save_contract token_contract $token_contract
        ;;
      governance)
        governance_contract=$(deploy deploy_governance)
        save_contract governance_contract $governance_contract
        ;;
      distribution)
        distribution_contract=$(deploy deploy_distribution)
        save_contract distribution_contract $distribution_contract
        ;;
      airdrop)
        airdrop_contract=$(deploy deploy_airdrop $token_contract)
        save_contract airdrop_contract $airdrop_contract
        ;;
      *)
        reserve_contract=$(deploy deploy_reserve)
        game_contract=$(deploy deploy_game $reserve_contract)
        ico_contract=$(deploy deploy_ico $reserve_contract)
        token_contract=$(deploy deploy_token $ico_contract)
        airdrop_contract=$(deploy deploy_airdrop $token_contract)
        save_contract reserve_contract $reserve_contract
        save_contract game_contract $game_contract
        save_contract ico_contract $ico_contract
        save_contract token_contract $token_contract
        save_contract airdrop_contract $airdrop_contract
        ;;
    esac
    ;;
  exec)
    read_contracts
    case $2 in 
      ico)
        exec_ico $3
        ;;
      game)
        exec_game $3
        ;;
      token)
        exec_token $3
        ;;
      governance)
        exec_governance $3
        ;;
      reserve)
        exec_reserve $3
        ;;
      distribution)
        exec_distribution $3 $4
        ;;
      *)
        echo "unknow exec contract"
        ;;
    esac
    ;;
  test)
    read_contracts
    case $2 in 
      ico)
        test_ico
        ;;
      reserve)
        test_reserve
        ;;
      *)
        echo "unknow input"
        ;;
    esac
    ;;
  print)
    read_contracts
    print_contracts
    ;;
  *)
    echo "unknow input"
    ;;
esac

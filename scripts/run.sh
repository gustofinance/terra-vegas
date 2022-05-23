: <<'END_COMMENT'
code_id=$(terrad tx wasm store ./artifacts/vegas_token_contract.wasm --from test1 --gas=auto --fees=222247uluna -y --chain-id localterra | grep -oP 'txhash: \K\w*' | xargs -i sh -c 'sleep 5; terrad query tx --type=hash --output json {} | jq -r ".logs[0].events[1].attributes[1].value"') \
&& echo $code_id > token_code_id.txt

code_id=$(terrad tx wasm store ./artifacts/governance_contract.wasm --from test1 --gas=auto --fees=222247uluna -y --chain-id localterra | grep -oP 'txhash: \K\w*' | xargs -i sh -c 'sleep 5; terrad query tx --type=hash --output json {} | jq -r ".logs[0].events[1].attributes[1].value"') \
&& echo $code_id > governance_code_id.txt

code_id=$(terrad tx wasm store ./artifacts/vegas_distribution_contract.wasm --from test1 --gas=auto --fees=222247uluna -y --chain-id localterra | grep -oP 'txhash: \K\w*' | xargs -i sh -c 'sleep 5; terrad query tx --type=hash --output json {} | jq -r ".logs[0].events[1].attributes[1].value"') \
&& echo $code_id > distribution_code_id.txt

code_id=$(terrad tx wasm store ./artifacts/reserve_contract.wasm --from test1 --gas=auto --fees=222247uluna -y --chain-id localterra | grep -oP 'txhash: \K\w*' | xargs -i sh -c 'sleep 5; terrad query tx --type=hash --output json {} | jq -r ".logs[0].events[1].attributes[1].value"') \
&& echo $code_id > reserve_code_id.txt

END_COMMENT

./scripts/test.sh deploy token
./scripts/test.sh deploy governance
./scripts/test.sh deploy reserve
./scripts/test.sh deploy distribution
./scripts/test.sh exec governance register_contract
./scripts/test.sh exec governance register_distribution
./scripts/test.sh exec token send
./scripts/test.sh exec reserve bank_send
./scripts/test.sh exec reserve add_game

#!/bin/bash

while true
do
export round=$(curl https://drand.cloudflare.com/public/latest |jq -r '.round')
export previous_signature=$(curl https://drand.cloudflare.com/public/latest |jq -r '.previous_signature' | xxd -r -p | base64 -w 0)
export signature=$(curl https://drand.cloudflare.com/public/latest |jq -r '.signature' | xxd -r -p | base64 -w 0)
echo $round
echo $previous_signature
echo $signature

echo $WALLET_PASSWORD | terrad tx wasm execute $TERRAND_CONTRACT '{"drand": {"round": '$round', "previous_signature": "'"$previous_signature"'", "signature": "'"$signature"'"}}'  --from terrand-woerker --chain-id $CHAIN_ID   --gas=auto --fees=1000000uusd --broadcast-mode=block --node=$NODE_URL -y


sleep 15
done

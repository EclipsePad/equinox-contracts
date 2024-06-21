# script for building contract

DIR_NAME=$(echo ${PWD##*/})
DIR_NAME_SNAKE=$(echo $DIR_NAME | tr '-' '_')
WASM="$DIR_NAME_SNAKE.wasm"
CODEGEN_PATH="./codegen"


# generate schema
cargo schema

# fix for ts-codegen MissingPointerError
# https://github.com/CosmWasm/ts-codegen/issues/90
rm -rf ./schema/raw

# generate contract-to-client interface
# cosmwasm-ts-codegen generate \
#   --plugin client \
#   --plugin react-query \
#   --optionalClient \
#   --version v4 \
#   --mutations \
#   --schema ./schema \
#   --out $CODEGEN_PATH \
#   --name $DIR_NAME \
#   --no-bundle
cosmwasm-ts-codegen generate \
  --plugin client \
  --optionalClient \
  --version v4 \
  --mutations \
  --schema ./schema \
  --out $CODEGEN_PATH \
  --name $DIR_NAME \
  --no-bundle

# build optimized binary
cd ../..
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.14.0

# check if contract is ready to be uploaded to the blockchain
if [ -e $WASM ]; then
    cosmwasm-check --available-capabilities iterator,stargate,staking $WASM
fi
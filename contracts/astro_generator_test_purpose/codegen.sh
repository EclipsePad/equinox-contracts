DIR_NAME=$(echo ${PWD##*/})
CODEGEN_PATH="./codegen"

# generate schema
cargo schema

# fix for ts-codegen MissingPointerError
# https://github.com/CosmWasm/ts-codegen/issues/90
rm -rf ./schema/raw

cosmwasm-ts-codegen generate \
  --plugin client \
  --plugin react-query \
  --optionalClient \
  --version v4 \
  --mutations \
  --schema ./schema \
  --out $CODEGEN_PATH \
  --name $DIR_NAME \
  --no-bundle

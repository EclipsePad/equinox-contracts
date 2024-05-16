DIR_NAME=$(echo ${PWD##*/})
CODEGEN_PATH="./codegen"
INTERFACES_PATH="../../scripts/neutron/src/interfaces"

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

cp -r "$CODEGEN_PATH/." $INTERFACES_PATH
find $INTERFACES_PATH -type f -name "*react-query.ts" -delete

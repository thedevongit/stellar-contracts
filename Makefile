default: build

# Variables
DSPONSOR_FACTORY = CAIFM7W2WMSIIDBPIACGG5FNXZ44DEPEYF7TDKIQ4BRNRT5E6VI33NWR
DSPONSOR_ADMIN = CDH3FBNCCBXJXVBME2CF4QZYS27RJFSUVXKRHD5DYVKCKQDCAK6UZBN3
DSPONSOR_MARKET = CCPJBIVAAXV2N3XNUO4IKILPPP3NDMFBBT7TABY2DO6ABOKSDRMKZJDM
NFT_CONTRACT = CCPO3TEPSH2JW6AMCDIUOZ7VNX3LQCFIJEZEAWI64PN5RDQLCUQ64ED6 # Just for testing purposes
SOURCE_ACCOUNT = siborg-mainnet
FEE_RECIPIENT = siborg-mainnet
NATIVE_XLM=CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA
NETWORK = mainnet

all: test deploy-factory deploy

test: build
	cargo test

build:
	cd dsponsor && cargo build --target wasm32v1-none --release
	cd dsponsor-factory && cargo build --target wasm32v1-none --release
	cd dsponsor-admin && cargo build --target wasm32v1-none --release
	cd dsponsor-market && cargo build --target wasm32v1-none --release

fmt:
	cargo fmt

clean:
	cargo clean
	rm -rf target/
	make build
	make test

deploy: 
	stellar contract deploy \
 	--wasm target/wasm32v1-none/release/dsponsor_admin.wasm \
	--source ${SOURCE_ACCOUNT} \
	--network ${NETWORK} 
	-- --simulate

deploy-factory: 
	stellar contract deploy \
 	--wasm target/wasm32v1-none/release/dsponsor_factory.wasm \
	--source ${SOURCE_ACCOUNT} \
	--network ${NETWORK} \

deploy-market:
	stellar contract deploy \
	 --wasm target/wasm32v1-none/release/dsponsor_market.wasm \
	 --source ${SOURCE_ACCOUNT} \
	 --network ${NETWORK} \

deploy-simple-nft: 
	stellar contract deploy \
 	--wasm target/wasm32v1-none/release/dsponsor.wasm \
	--source ${SOURCE_ACCOUNT} \
	--network ${NETWORK} \

# Initialize the contract
initialize:
	stellar contract invoke \
		--id $(DSPONSOR_ADMIN) \
		--source $(SOURCE_ACCOUNT) \
		--network $(NETWORK) \
		-- \
		initialize \
		--nft_factory $(DSPONSOR_FACTORY) \
		--fee_recipient $(FEE_RECIPIENT) \
		--native_xlm $(NATIVE_XLM) \
		--bps 100 \
		--admin $(SOURCE_ACCOUNT) \

# Initialize the market contract
initialize-market:
	stellar contract invoke \
		--id $(DSPONSOR_MARKET) \
		--source $(SOURCE_ACCOUNT) \
		--network $(NETWORK) \
		-- \
		initialize \
		--admin $(SOURCE_ACCOUNT) \
		--native_xlm $(NATIVE_XLM) \
		--fee_bps 100 \

create_offer:
	stellar contract invoke \
		--id $(DSPONSOR_ADMIN) \
		--source $(SOURCE_ACCOUNT) \
		--network $(NETWORK) \
		-- \
		create_dsponsor_nft_and_offer \
		

create_nft_contract:
	stellar contract invoke \
		--id $(DSPONSOR_ADMIN) \
		--source $(SOURCE_ACCOUNT) \
		--network $(NETWORK) \
		-- \
		create_dsponsor_nft \
		--init_params $() \
		--native_xlm $(NATIVE_XLM) \
		--salt 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef \



get_offers:
	stellar contract invoke \
		--id $(DSPONSOR_ADMIN) \
		--source $(SOURCE_ACCOUNT) \
		--network $(NETWORK) \
		-- \
		get_all_offers \

get_nft_tokens:
	stellar contract invoke \
		--id $(NFT_CONTRACT) \
		--source $(SOURCE_ACCOUNT) \
		--network $(NETWORK) \
		-- \
		get_user_tokens \
		--user GDBCM52A7NMCHYYCWE3H52DP42RZEISOPUQWE4PQZ7JL3OLFL2UBTKER \

approve_native_xlm:
	stellar contract invoke \
		--id $(NATIVE_XLM) \
		--source $(SOURCE_ACCOUNT) \
		--network $(NETWORK) \
		-- \
		approve \
		--from $(SOURCE_ACCOUNT) \
		--spender $(DSPONSOR_ADMIN) \
		--amount 1000000000000000000 \
		--expiration-ledger 
		
sdk-gen: 
	soroban contract bindings typescript \
  --rpc-url https://soroban-testnet.stellar.org:443 \
  --network-passphrase "Test SDF Network ; September 2015" \
  --contract-id $(DSPONSOR_ADMIN) \
  --output-dir ../sdk \
  --overwrite


admin-ready: 
	make initialize
	make sdk-gen

.PHONY: default all test build build-debug fmt clean


# nft testing:

nft-allowance:
	stellar contract invoke \
		--id $(NFT_CONTRACT) \
		--source $(SOURCE_ACCOUNT) \
		--network $(NETWORK) \
		-- \
		is_approved \
		--operator $(DSPONSOR_MARKET) \
		--token_id 3 \

# soroban contract asset id --asset native --network testnet
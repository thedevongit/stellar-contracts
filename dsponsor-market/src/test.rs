#![cfg(test)]

use soroban_sdk::{
    testutils::Address as _, token, vec, Address, Env, String,
};
use token::Client as TokenClient;
use token::StellarAssetClient as TokenAdminClient;

use crate::{DSponsorMarket, DSponsorMarketClient};
use crate::dsponsor::WASM as DSPONSOR_WASM;
use crate::dsponsor::Client as DSponsorNFTClient;
use crate::dsponsor::InitParams as DSponsorInitParams;
use crate::dsponsor::MintPriceSettings as DSponsorMintPriceSettings;

// Types mirroring dsponsor InitParams for constructor call

fn create_token_contract<'a>(e: &Env, admin: &Address) -> (TokenClient<'a>, TokenAdminClient<'a>) {
    let sac = e.register_stellar_asset_contract_v2(admin.clone());
    (
        token::Client::new(e, &sac.address()),
        token::StellarAssetClient::new(e, &sac.address()),
    )
}

fn setup_market(env: &Env) -> (Address, DSponsorMarketClient) {
    let id = env.register(DSponsorMarket, ());
    let client = DSponsorMarketClient::new(env, &id);
    (id, client)
}

fn setup_dsponsor(
    env: &Env,
    minter: Address,
    admin: Address,
    native_xlm: Address,
) -> (Address, DSponsorNFTClient) {
    let init_params = DSponsorInitParams {
        name: String::from_str(env, "Test NFT"),
        symbol: String::from_str(env, "TNFT"),
        base_uri: String::from_str(env, "base_uri"),
        contract_uri: String::from_str(env, "contract_uri"),
        minter: minter.clone(),
        max_supply: 100,
        forwarder: Address::generate(env),
        initial_owner: admin.clone(),
        royalty_bps: 500,
        currencies: vec![env],
        prices: vec![env],
        allowed_token_ids: vec![env, 1u32],
        apply_tokens_allowlist: true,
        default_native_price: DSponsorMintPriceSettings {
            enabled: true,
            amount: 0u128, // free mint for tests
        },
    };

     // Register contract with constructor arguments
    let nft = env.register(DSPONSOR_WASM, (init_params.clone(), native_xlm.clone()));
    let client = DSponsorNFTClient::new(env, &nft);
    (nft, client)
}

#[test]
fn test_listing_buy_native() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);

    // Token setup (used as native_xlm for market and NFT)
    let (token_client, token_admin) = create_token_contract(&env, &admin);
    token_admin.mint(&buyer, &10_000); // fund buyer

    // Deploy market and initialize
    let (_market_id, market) = setup_market(&env);
    market.initialize(&admin, &token_client.address, &50); // 5% fee

    // Deploy dsponsor NFT and mint token 1 to seller (free)
    let (_nft_addr, nft) = setup_dsponsor(&env, seller.clone(), seller.clone(), token_client.address.clone());
    // Approve token spend for free mint path is not needed since amount=0
    nft.mint(&seller, &1i128, &seller, &token_client.address);

    // Approve market as spender to transfer NFT
    nft.approve(&seller, &market.address, &1i128);

    // Create listing and buy
    let listing_id = market.create_listing(&seller, &nft.address, &1i128, &token_client.address, &1_000u128);
    market.buy(&listing_id, &buyer);

    assert_eq!(nft.owner_of(&1i128), Some(buyer.clone()));

    // Check balances: buyer -1000, seller +950, admin +50
    assert_eq!(token_client.balance(&buyer), 9_000);
    assert_eq!(token_client.balance(&seller), 950);
    assert_eq!(token_client.balance(&admin), 50);
}

#[test]
fn test_auction_bid_refund_and_finalize() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let bidder1 = Address::generate(&env);
    let bidder2 = Address::generate(&env);

    // Token setup
    let (token_client, token_admin) = create_token_contract(&env, &admin);
    token_admin.mint(&bidder1, &5_000);
    token_admin.mint(&bidder2, &5_000);

    // Deploy market and initialize
    let (_market_id, market) = setup_market(&env);
    market.initialize(&admin, &token_client.address, &100); // 10% fee

    // Deploy dsponsor NFT and mint token 1 to seller (free)
    let (_nft_addr, nft) = setup_dsponsor(&env, seller.clone(), seller.clone(), token_client.address.clone());
    nft.mint(&seller, &1i128, &seller, &token_client.address);
    // Approve market as spender to transfer NFT
    nft.approve(&seller, &market.address, &1i128);

    // Create auction
    let auction_id = market.create_auction(&seller, &nft.address, &1i128, &token_client.address, &500u128);

    // Place bids: bidder1 600, bidder2 800 (outbid and refund bidder1)
    market.bid(&auction_id, &bidder1, &600u128);
    assert_eq!(token_client.balance(&bidder1), 4_400); // escrowed 600
    market.bid(&auction_id, &bidder2, &800u128);
    // bidder1 refunded 600
    assert_eq!(token_client.balance(&bidder1), 5_000);
    assert_eq!(token_client.balance(&bidder2), 4_200); // escrowed 800

    // Finalize by seller
    market.finalize_auction(&auction_id, &seller);
    assert_eq!(nft.owner_of(&1i128), Some(bidder2.clone()));

    // Payouts: 10% fee of 800 = 80, seller gets 720
    assert_eq!(token_client.balance(&seller), 720);
    assert_eq!(token_client.balance(&admin), 80);
    // bidder2 paid 800 total
    assert_eq!(token_client.balance(&bidder2), 4_200);
}

#[test]
fn test_get_all_listings_and_auctions() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let seller = Address::generate(&env);

    // Token setup
    let (token_client, _token_admin) = create_token_contract(&env, &admin);

    // Deploy market and initialize
    let (_market_id, market) = setup_market(&env);
    market.initialize(&admin, &token_client.address, &50); // 5% fee

    // Deploy dsponsor NFT and mint one token
    let (_nft_addr, nft) = setup_dsponsor(&env, seller.clone(), seller.clone(), token_client.address.clone());
    nft.mint(&seller, &1i128, &seller, &token_client.address);

    // Approve market as spender
    nft.approve(&seller, &market.address, &1i128);

    // Initially no listings or auctions
    let all_listings = market.get_all_listings();
    let all_auctions = market.get_all_auctions();
    assert_eq!(all_listings.len(), 0);
    assert_eq!(all_auctions.len(), 0);

    // Create one listing
    let listing_id = market.create_listing(&seller, &nft.address, &1i128, &token_client.address, &1_000u128);

    // Check all listings
    let all_listings = market.get_all_listings();
    assert_eq!(all_listings.len(), 1);
    assert_eq!(all_listings.get(0).unwrap().id, 1);

    // Check all auctions (should still be empty)
    let all_auctions = market.get_all_auctions();
    assert_eq!(all_auctions.len(), 0);

    // Cancel the listing and check it's not returned
    market.cancel_listing(&listing_id, &seller);
    let all_listings = market.get_all_listings();
    assert_eq!(all_listings.len(), 0); // No active listings
}



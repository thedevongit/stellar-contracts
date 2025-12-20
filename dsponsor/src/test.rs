#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _}, token, vec, Address, Env, String, 
};
use token::Client as TokenClient;
use token::StellarAssetClient as TokenAdminClient;

use crate::{DSponsorNFT,DSponsorNFTClient, InitParams, MintPriceSettings};

fn create_token_contract<'a>(e: &Env, admin: &Address) -> (TokenClient<'a>, TokenAdminClient<'a>) {
    let sac = e.register_stellar_asset_contract_v2(admin.clone());
    (
        token::Client::new(e, &sac.address()),
        token::StellarAssetClient::new(e, &sac.address()),
    )
}

// Helper function to create a test environment
fn setup_test_env() -> (Env, Address, Address, Address, Address, DSponsorNFTClient<'static>) {
    let env = Env::default();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let minter = Address::generate(&env);
    
    // Create token contract for native XLM
    let (_, native_xlm_admin) = create_token_contract(&env, &user);
    env.mock_all_auths();
    native_xlm_admin.mint(&minter, &10000);
    let native_xlm = native_xlm_admin.address;

    // Create initialization parameters
    let init_params = InitParams {
        name: String::from_str(&env, "Test NFT"),
        symbol: String::from_str(&env, "TEST"),
        base_uri: String::from_str(&env, "https://test.com/"),
        contract_uri: String::from_str(&env, "https://test.com/contract"),
        minter: minter.clone(),
        max_supply: 1000,
        forwarder: Address::generate(&env),
        initial_owner: admin.clone(),
        royalty_bps: 500,
        currencies: vec![&env, native_xlm.clone()],
        prices: vec![&env, 1000],
        allowed_token_ids: vec![&env, 1, 2, 3],
        apply_tokens_allowlist: true,
        default_native_price: MintPriceSettings {
            enabled: true,
            amount: 1000,
        },
    };

    // Register contract with constructor arguments
    let contract_id = env.register(DSponsorNFT, (init_params, native_xlm.clone()));
    let client = DSponsorNFTClient::new(&env, &contract_id);

    (env, admin, user, minter, native_xlm, client)
}


#[test]
fn test_name() {
    let (env, _, _, _, _, client) = setup_test_env();
    assert_eq!(client.name(), Some(String::from_str(&env, "Test NFT")));
}

#[test]
fn test_symbol() {
    let (env, _, _, _, _, client) = setup_test_env();
    assert_eq!(client.symbol(), Some(String::from_str(&env, "TEST")));
}

#[test]
fn test_owner_of() {
    let (env, _, user, minter, native_xlm, client) = setup_test_env();
    
    let token = token::Client::new(&env, &native_xlm);
    // Approve the contract to spend user's tokens
    token.approve(&minter, &client.address, &10000, &1000);
    env.mock_all_auths();
        
    // Mint a token first
    client.mint(&minter, &1, &user, &native_xlm);
    
    // Check owner
    assert_eq!(client.owner_of(&1), Some(user));
}

#[test]
fn test_token_uri() {
    let (env, _, user, minter, native_xlm, client) = setup_test_env();
    
    // Get token client for native XLM
    let token = token::Client::new(&env, &native_xlm);
    // Approve the contract to spend user's tokens
    token.approve(&minter, &client.address, &10000, &1000);
    env.mock_all_auths();
    
    // Mint a token first
    client.mint(&minter, &1, &user, &native_xlm);
    
    // Check URI - it should return the contract URI
    assert_eq!(client.token_uri(&1), String::from_str(&env, "https://test.com/contract"));
}

#[test]
fn test_token_image() {
    let (env, _, _, _, _, client) = setup_test_env();
    assert_eq!(client.token_image(), String::from_str(&env, "https://test.com/"));
}

#[test]
fn test_is_approved() {
    let (env, _, user, minter, native_xlm, client) = setup_test_env();
    let operator = Address::generate(&env);
    
    let token = token::Client::new(&env, &native_xlm);
    // Approve the contract to spend user's tokens
    token.approve(&minter, &client.address, &10000, &1000);
    env.mock_all_auths();
        
    // Mint a token first
    client.mint(&minter, &1, &user, &native_xlm);
    
    // Initially not approved
    assert_eq!(client.is_approved(&operator, &1), false);
    
    // Approve operator
    client.approve(&user, &operator, &1);
    
    // Now approved
    assert_eq!(client.is_approved(&operator, &1), true);
}

#[test]
fn test_get_owner() {
    let (_, admin, _, _, _, client) = setup_test_env();
    assert_eq!(client.get_owner(), admin);
}

#[test]
fn test_get_mint_price() {
    let (_, _, _, _, native_xlm, client) = setup_test_env();
    
    // Check mint price for token ID 1 with native XLM
    let price_settings = client.get_mint_price(&1, &native_xlm);
    assert_eq!(price_settings.enabled, true);
    assert_eq!(price_settings.amount, 1000);
}

#[test]
fn test_get_token_count() {
    let (env, _, user, minter, native_xlm, client) = setup_test_env();
    
    // Initially 0
    assert_eq!(client.get_token_count(), 0);
    
    // Get token client for native XLM
    let token = token::Client::new(&env, &native_xlm);
    // Approve the contract to spend user's tokens
    token.approve(&minter, &client.address, &10000, &1000);
    // env.mock_all_auths();
    
    // Mint a token
    client.mint(&minter, &1, &user, &native_xlm);
    
    // Now 1
    assert_eq!(client.get_token_count(), 1);
}

#[test]
fn test_is_token_id_allowed() {
    let (_, _, _, _, _, client) = setup_test_env();
    
    // Token ID 1 is in the allowed list
    assert_eq!(client.is_token_id_allowed(&1), true);
    
    // Token ID 999 is not in the allowed list
    assert_eq!(client.is_token_id_allowed(&999), false);
}
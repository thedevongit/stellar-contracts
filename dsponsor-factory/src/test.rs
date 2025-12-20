#![cfg(test)]

use soroban_sdk::{
    testutils::Address as _,
    vec, Address, Env, IntoVal, Symbol, Vec, String, BytesN,
};

use crate::{DSponsorFactory, dsponsor};

// Helper function to create a test environment
fn setup_test_env() -> (Env, Address, Address) {
    let env = Env::default();
    
    // Create test addresses
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    
    (env, admin, user)
}

// Helper function to create a test DSponsorFactory contract
fn setup_factory(env: &Env, _admin: &Address) -> Address {
    // Register the contract and get its ID
    env.register(DSponsorFactory, ())
}

// Helper function to create test initialization parameters for DSponsorNFT
fn create_test_init_params(env: &Env, admin: &Address, _user: &Address) -> (dsponsor::InitParams, Address) {
    let name = String::from_str(env, "Test NFT");
    let symbol = String::from_str(env, "TNFT");
    let base_uri = String::from_str(env, "https://example.com/nft/");
    let contract_uri = String::from_str(env, "https://example.com/contract");
    let minter = admin.clone();
    let max_supply = 1000u32;
    let forwarder = Address::generate(env);
    let initial_owner = admin.clone();
    let royalty_bps = 500u32;
    let currencies = vec![env, admin.clone()];
    let prices = vec![env, 1000u128];
    let allowed_token_ids = vec![env, 1u32, 2u32, 3u32];
    let apply_tokens_allowlist = true;
    let default_native_price = dsponsor::MintPriceSettings {
        enabled: true,
        amount: 1000u128,
    };
    
    let init_params = dsponsor::InitParams {
        name,
        symbol,
        base_uri,
        contract_uri,
        minter,
        max_supply,
        forwarder,
        initial_owner,
        royalty_bps,
        currencies,
        prices,
        allowed_token_ids,
        apply_tokens_allowlist,
        default_native_price,
    };
    
    let native_xlm = Address::generate(env);
    
    (init_params, native_xlm)
}

#[test]
fn test_create_dsponsor_nft() {
    // Setup test environment
    let (env, admin, user) = setup_test_env();
    let factory = setup_factory(&env, &admin);
    
    // Create test initialization parameters
    let (init_params, native_xlm) = create_test_init_params(&env, &admin, &user);
    
    // Generate a random salt for the deployment
    let mut salt_bytes = [0u8; 32];
    for i in 0..32 {
        salt_bytes[i] = (i + 1) as u8;
    }
    let salt = Some(BytesN::from_array(&env, &salt_bytes));
    
    // Create a DSponsorNFT contract
    let nft_contract = env.invoke_contract::<Address>(
        &factory,
        &Symbol::new(&env, "create_dsponsor_nft"),
        vec![&env, init_params.into_val(&env), native_xlm.into_val(&env), salt.into_val(&env)],
    );
    
    // Verify the contract was deployed
    assert!(nft_contract != Address::generate(&env));
    
    // Verify the contract has the correct name and symbol
    let nft_client = dsponsor::Client::new(&env, &nft_contract);
    assert_eq!(nft_client.name(), Some(String::from_str(&env, "Test NFT")));
    assert_eq!(nft_client.symbol(), Some(String::from_str(&env, "TNFT")));
}

#[test]
fn test_create_dsponsor_nft_with_random_salt() {
    // Setup test environment
    let (env, admin, user) = setup_test_env();
    let factory = setup_factory(&env, &admin);
    
    // Create test initialization parameters
    let (init_params, native_xlm) = create_test_init_params(&env, &admin, &user);
    
    // Use None to let the contract generate a random salt
    let salt: Option<BytesN<32>> = None;
    
    // Create a DSponsorNFT contract
    let nft_contract = env.invoke_contract::<Address>(
        &factory,
        &Symbol::new(&env, "create_dsponsor_nft"),
        vec![&env, init_params.into_val(&env), native_xlm.into_val(&env), salt.into_val(&env)],
    );
    
    // Verify the contract was deployed
    assert!(nft_contract != Address::generate(&env));
    
    // Verify the contract has the correct name and symbol
    let nft_client = dsponsor::Client::new(&env, &nft_contract);
    assert_eq!(nft_client.name(), Some(String::from_str(&env, "Test NFT")));
    assert_eq!(nft_client.symbol(), Some(String::from_str(&env, "TNFT")));
}

#[test]
fn test_create_multiple_dsponsor_nfts() {
    // Setup test environment
    let (env, admin, user) = setup_test_env();
    let factory = setup_factory(&env, &admin);
    
    // Create test initialization parameters
    let (init_params1, native_xlm1) = create_test_init_params(&env, &admin, &user);
    
    // Create modified parameters for the second contract
    let name = String::from_str(&env, "Test NFT 2");
    let symbol = String::from_str(&env, "TNFT2");
    let base_uri = String::from_str(&env, "https://example.com/nft/");
    let contract_uri = String::from_str(&env, "https://example.com/contract");
    let minter = admin.clone();
    let max_supply = 1000u32;
    let forwarder = Address::generate(&env);
    let initial_owner = admin.clone();
    let royalty_bps = 500u32;
    let currencies: Vec<Address> = vec![&env, admin.clone()];
    let prices: Vec<u128> = vec![&env, 1000u128];
    let allowed_token_ids: Vec<u32> = vec![&env, 1u32, 2u32, 3u32];
    let apply_tokens_allowlist = true;
    let default_native_price = dsponsor::MintPriceSettings {
        enabled: true,
        amount: 1000u128,
    };
    
    let init_params2 = dsponsor::InitParams {
        name,
        symbol,
        base_uri,
        contract_uri,
        minter,
        max_supply,
        forwarder,
        initial_owner,
        royalty_bps,
        currencies,
        prices,
        allowed_token_ids,
        apply_tokens_allowlist,
        default_native_price,
    };
    
    let native_xlm2 = Address::generate(&env);
    
    // Create two DSponsorNFT contracts with different salt values to ensure unique addresses
    let salt1 = BytesN::from_array(&env, &[1; 32]);
    let salt2 = BytesN::from_array(&env, &[2; 32]);
    
    // Create the first NFT contract
    let nft_contract1 = env.invoke_contract::<Address>(
        &factory,
        &Symbol::new(&env, "create_dsponsor_nft"),
        vec![&env, init_params1.into_val(&env), native_xlm1.into_val(&env), Some(salt1).into_val(&env)],
    );
    
    // Create the second NFT contract
    let nft_contract2 = env.invoke_contract::<Address>(
        &factory,
        &Symbol::new(&env, "create_dsponsor_nft"),
        vec![&env, init_params2.into_val(&env), native_xlm2.into_val(&env), Some(salt2).into_val(&env)],
    );
    
    // Verify the contracts were deployed with different addresses
    assert!(nft_contract1 != nft_contract2);
    
    // Verify the contracts have the correct names and symbols
    let nft_client1 = dsponsor::Client::new(&env, &nft_contract1);
    let nft_client2 = dsponsor::Client::new(&env, &nft_contract2);
    
    assert_eq!(nft_client1.name(), Some(String::from_str(&env, "Test NFT")));
    assert_eq!(nft_client1.symbol(), Some(String::from_str(&env, "TNFT")));
    
    assert_eq!(nft_client2.name(), Some(String::from_str(&env, "Test NFT 2")));
    assert_eq!(nft_client2.symbol(), Some(String::from_str(&env, "TNFT2")));
}

#[test]
fn test_create_dsponsor_nft_with_different_parameters() {
    // Setup test environment
    let (env, admin, user) = setup_test_env();
    let factory = setup_factory(&env, &admin);
    
    // Create test initialization parameters with different values
    let name = String::from_str(&env, "Custom NFT");
    let symbol = String::from_str(&env, "CNFT");
    let base_uri = String::from_str(&env, "https://custom.com/nft/");
    let contract_uri = String::from_str(&env, "https://custom.com/contract");
    let minter = user.clone();
    let max_supply = 500u32;
    let forwarder = Address::generate(&env);
    let initial_owner = user.clone();
    let royalty_bps = 250u32;
    let currencies: Vec<Address> = vec![&env, admin.clone(), user.clone()];
    let prices: Vec<u128> = vec![&env, 500u128, 1000u128];
    let allowed_token_ids: Vec<u32> = vec![&env, 1u32, 2u32];
    let apply_tokens_allowlist = false;
    let default_native_price = dsponsor::MintPriceSettings {
        enabled: true,
        amount: 500u128,
    };
    
    let init_params = dsponsor::InitParams {
        name,
        symbol,
        base_uri,
        contract_uri,
        minter,
        max_supply,
        forwarder,
        initial_owner,
        royalty_bps,
        currencies,
        prices,
        allowed_token_ids,
        apply_tokens_allowlist,
        default_native_price,
    };
    
    let native_xlm = Address::generate(&env);
    
    // Generate a different random salt for this deployment
    let mut salt_bytes = [0u8; 32];
    for i in 0..32 {
        salt_bytes[i] = (i + 100) as u8; // Different pattern than the first test
    }
    let salt = Some(BytesN::from_array(&env, &salt_bytes));
    
    // Create a DSponsorNFT contract with custom parameters
    let nft_contract = env.invoke_contract::<Address>(
        &factory,
        &Symbol::new(&env, "create_dsponsor_nft"),
        vec![&env, init_params.into_val(&env), native_xlm.into_val(&env), salt.into_val(&env)],
    );
    
    // Verify the contract was deployed
    assert!(nft_contract != Address::generate(&env));
    
    // Verify the contract has the correct custom parameters
    let nft_client = dsponsor::Client::new(&env, &nft_contract);
    assert_eq!(nft_client.name(), Some(String::from_str(&env, "Custom NFT")));
    assert_eq!(nft_client.symbol(), Some(String::from_str(&env, "CNFT")));
    assert_eq!(nft_client.get_owner(), user);
}

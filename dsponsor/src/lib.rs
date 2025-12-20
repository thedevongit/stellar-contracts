#![no_std]

/*
  ___              _               _  _ ___ _____
 / __| ___ _ _ ___| |__  __ _ _ _ | \| | __|_   _|
 \__ \/ _ \ '_/ _ \ '_ \/ _` | ' \| .` | _|  | |
 |___/\___/_| \___/_.__/\__,_|_||_|_|\_|_|   |_|
     - Released under 3N consideration -
*/

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token, Address, Env, String, Symbol,
    Vec,
};

#[contract]
pub struct DSponsorNFT;

// Storage symbols for instance storage
const NAME: Symbol = symbol_short!("NAME");
const SYMBOL: Symbol = symbol_short!("SYMBOL");
const BASE_URI: Symbol = symbol_short!("BASE_URI");
const CONT_URI: Symbol = symbol_short!("CONT_URI");
const MINTER: Symbol = symbol_short!("MINTER");
const MAX_SUP: Symbol = symbol_short!("MAX_SUP");
const FORWARD: Symbol = symbol_short!("FORWARD");
const INIT_OWN: Symbol = symbol_short!("INIT_OWN");
const ROY_BPS: Symbol = symbol_short!("ROY_BPS");
const CURREN: Symbol = symbol_short!("CURREN");
const PRICES: Symbol = symbol_short!("PRICES");
const ALL_TK: Symbol = symbol_short!("ALL_TK");
const TOKEN_CNT: Symbol = symbol_short!("TOKEN_CNT");
const APPLY_ALLOW: Symbol = symbol_short!("ALLOW");
const DEF_NAT_PRICE: Symbol = symbol_short!("DEF_NAT");
const NATIVE: Symbol = symbol_short!("NATIVE");
const ADMIN: Symbol = symbol_short!("admin");

// DataKey enum for persistent storage
#[contracttype]
pub enum DataKey {
    Owner(i128),
    TokenCount,
    Approvals(i128),
    TokenURI(i128),
    AllowedTokenId(i128),
    DefERC20Price(Address),
    ERC20Price(i128, Address),
    NatPrice(i128),
    Renter(i128),
}

#[contracttype]
#[derive(Clone)]
struct RentalInfo {
    user: Address,
    duration: u64
}

// Structure for mint price settings
#[contracttype]
#[derive(Clone)]
pub struct MintPriceSettings {
    pub enabled: bool,
    pub amount: u128,
}

#[contracttype]
#[derive(Clone)]
pub struct InitParams {
    pub name: String,
    pub symbol: String,
    pub base_uri: String,
    pub contract_uri: String,
    pub minter: Address,
    pub max_supply: u32,
    pub forwarder: Address,
    pub initial_owner: Address,
    pub royalty_bps: u32,
    pub currencies: Vec<Address>,
    pub prices: Vec<u128>,
    pub allowed_token_ids: Vec<u32>,
    pub apply_tokens_allowlist: bool,
    pub default_native_price: MintPriceSettings,
}

#[contractimpl]
impl DSponsorNFT {
    pub fn __constructor(env: Env, init_params: InitParams, native_xlm: Address) {
        // Store initialization parameters
        env.storage().instance().set(&NAME, &init_params.name);
        env.storage().instance().set(&SYMBOL, &init_params.symbol);
        env.storage()
            .instance()
            .set(&BASE_URI, &init_params.base_uri);
        env.storage()
            .instance()
            .set(&CONT_URI, &init_params.contract_uri);
        env.storage().instance().set(&MINTER, &init_params.minter);
        env.storage()
            .instance()
            .set(&MAX_SUP, &init_params.max_supply);
        env.storage()
            .instance()
            .set(&FORWARD, &init_params.forwarder);
        env.storage()
            .instance()
            .set(&INIT_OWN, &init_params.initial_owner);
        env.storage()
            .instance()
            .set(&ROY_BPS, &init_params.royalty_bps);
        env.storage()
            .instance()
            .set(&CURREN, &init_params.currencies);
        env.storage().instance().set(&PRICES, &init_params.prices);
        env.storage()
            .instance()
            .set(&ALL_TK, &init_params.allowed_token_ids);
        env.storage()
            .instance()
            .set(&APPLY_ALLOW, &init_params.apply_tokens_allowlist);
        env.storage()
            .instance()
            .set(&DEF_NAT_PRICE, &init_params.default_native_price);
        env.storage().persistent().set(&NATIVE, &native_xlm);
        env.storage()
            .persistent()
            .set(&ADMIN, &init_params.initial_owner);
        // Initialize token count to 0
        env.storage().instance().set(&TOKEN_CNT, &0i128);

        // Initialize allowed token IDs
        for i in 0..init_params.allowed_token_ids.len() {
            Self::_set_tokens_allowlist(&env, true);
            Self::_set_token_allowlist(
                &env,
                i128::from(init_params.allowed_token_ids.get(i).unwrap()),
                true,
            );
        }

        // Set default mint prices
        for i in 0..init_params.currencies.len() {
            let currency = init_params.currencies.get(i).unwrap();
            let price = init_params.prices.get(i).unwrap();
            Self::_set_default_mint_price(&env, &currency, true, price);
        }

        // Log initialization
        env.events().publish(
            (symbol_short!("Init"),),
            (init_params.name, init_params.symbol),
        );
    }

    /* ****************
     *  Soroban nft similar to erc721 functions
     *****************/

    pub fn mint(env: Env, caller: Address, token_id: i128, to: Address, currency: Address) {
        caller.require_auth();
        // Check if token is already minted
        let token_count = env.storage().instance().get(&TOKEN_CNT).unwrap_or(0);
        if token_id <= token_count {
            panic!("Token already minted");
        }

        // Check if token is allowed
        let apply_allowlist = env
            .storage()
            .instance()
            .get::<Symbol, bool>(&APPLY_ALLOW)
            .unwrap_or(false);
        if apply_allowlist && !Self::is_token_id_allowed(env.clone(), token_id) {
            panic!("Token not allowed");
        }

        let mut paid_amount: u128 = 0;
        let admin = env
            .storage()
            .persistent()
            .get::<Symbol, Address>(&ADMIN)
            .unwrap();
        let current_contract_address = env.current_contract_address();
        let recipient: Address = Self::get_owner(env.clone());
        // If sender is not the admin, check authorization and payment
        if caller != admin {
            let minter = env
                .storage()
                .instance()
                .get::<Symbol, Address>(&MINTER)
                .unwrap();
            if !Some(minter.clone()).is_none() && caller != minter {
                panic!("Unauthorized to mint");
            }

            // Get mint price settings
            let price_settings = Self::get_mint_price(env.clone(), token_id, currency.clone());
            if !price_settings.enabled {
                panic!("Forbidden currency");
            }
            let native_address = env
                .storage()
                .persistent()
                .get::<Symbol, Address>(&NATIVE)
                .unwrap();
            // Process payment if amount > 0
            if price_settings.amount > 0 {
                // For native currency (XLM)
                let amount_i128: i128 = price_settings.amount as i128;
                if currency == native_address {
                    // In Soroban, we can't directly access msg.value like in Solidity
                    // We need to handle native payments differently
                    let token_client = token::TokenClient::new(&env, &native_address);
                    token_client.transfer_from(
                        &current_contract_address,
                        &caller,
                        &recipient,
                        &amount_i128,
                    );
                } else {
                    // For ERC20 tokens, we need to transfer from the caller to the admin
                    // This would require an ERC20 token contract implementation
                    let token_client = token::TokenClient::new(&env, &currency);
                    token_client.transfer_from(
                        &current_contract_address,
                        &caller,
                        &recipient,
                        &amount_i128,
                    );
                }
                paid_amount = price_settings.amount;
            }
        }

        // Mint the token
        Self::_safemint(env.clone(),token_id, to.clone());

        // Publish mint event
        env.events().publish(
            (symbol_short!("Mint"),),
            (token_id, caller.clone(), to.clone(), currency.clone(), paid_amount),
        );
    }

    pub fn owner_of(env: Env, token_id: i128) -> Option<Address> {
        let key = DataKey::Owner(token_id);
        env.storage().persistent().get(&key).unwrap_or_default()
    }

    pub fn name(env: Env) -> Option<String> {
        env.storage().instance().get(&NAME).unwrap_or_default()
    }

    pub fn symbol(env: Env) -> Option<String> {
        env.storage().instance().get(&SYMBOL).unwrap_or_default()
    }

    pub fn token_uri(env: Env, token_id: i128) -> String {
        // First check if there's a custom URI for this token
        let key = DataKey::TokenURI(token_id);
        let custom_uri = env.storage().persistent().get::<DataKey, String>(&key);

        if let Some(uri) = custom_uri {
            return uri;
        }

        // If no custom URI, return the contract URI
        env.storage()
            .instance()
            .get(&CONT_URI)
            .unwrap_or_else(|| String::from_str(&env, ""))
    }

    pub fn token_image(env: Env) -> String {
        env.storage()
            .instance()
            .get(&BASE_URI)
            .unwrap_or_else(|| String::from_str(&env, ""))
    }

    pub fn is_approved(env: Env, operator: Address, token_id: i128) -> bool {
        let key = DataKey::Approvals(token_id);
        let approvals = env
            .storage()
            .persistent()
            .get::<DataKey, Vec<Address>>(&key)
            .unwrap_or_else(|| Vec::new(&env));
        approvals.contains(&operator)
    }

    pub fn transfer(env: Env, owner: Address, to: Address, token_id: i128) {
        owner.require_auth();
        let actual_owner = Self::owner_of(env.clone(), token_id).unwrap();
        if owner == actual_owner {
            let owner_key = DataKey::Owner(token_id);
            let appr_key = DataKey::Approvals(token_id);

            env.storage().persistent().set(&owner_key, &to);
            // Clear any existing rental and set new owner as user
            let renter_key = DataKey::Renter(token_id);
            env.storage().persistent().remove(&renter_key);
            let expiry_time = env.ledger().timestamp() + 3153600000000;  // as long as possible
            Self::__set_user(env.clone(), to.clone(), token_id, to.clone(), expiry_time);
            env.storage().persistent().remove(&appr_key);
            env.events()
                .publish((symbol_short!("Transfer"),), (owner, to, token_id));
        } else {
            panic!("Not the token owner");
        }
    }

    pub fn approve(env: Env, owner: Address, to: Address, token_id: i128) {
        owner.require_auth();
        let actual_owner = Self::owner_of(env.clone(), token_id).unwrap();
        if owner == actual_owner {
            let key = DataKey::Approvals(token_id);
            let mut approvals = env
                .storage()
                .persistent()
                .get::<DataKey, Vec<Address>>(&key)
                .unwrap_or_else(|| Vec::new(&env));
            if !approvals.contains(&to) {
                approvals.push_back(to.clone());
                env.storage().persistent().set(&key, &approvals);
                env.events()
                    .publish((symbol_short!("Approval"),), (owner, to, token_id));
            }
        } else {
            panic!("Not the token owner");
        }
    }

    pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, token_id: i128) {
        spender.require_auth();
        let actual_owner = Self::owner_of(env.clone(), token_id).unwrap();
        if from != actual_owner {
            panic!("From not owner");
        }

        let key = DataKey::Approvals(token_id);
        let approvals = env
            .storage()
            .persistent()
            .get::<DataKey, Vec<Address>>(&key)
            .unwrap_or_else(|| Vec::new(&env));
        if !approvals.contains(&spender) {
            panic!("Spender is not approved for this token");
        }
        let owner_key = DataKey::Owner(token_id);
        env.storage().persistent().set(&owner_key, &to);
        // Clear any existing rental and set new owner as user
        let renter_key = DataKey::Renter(token_id);
        env.storage().persistent().remove(&renter_key);
        let expiry_time = env.ledger().timestamp() + 3153600000000;  // as long as possible
        Self::__set_user(env.clone(), to.clone(), token_id, to.clone(), expiry_time);
        env.storage().persistent().remove(&key);

        env.events()
            .publish((symbol_short!("Transfer"),), (from, to, token_id));
    }

    // IERC4907 rental nft functions
    // private __set_user function
    fn __set_user(env: Env, caller: Address, token_id: i128, to: Address, duration: u64) {
        let owner = Self::owner_of(env.clone(), token_id).unwrap();
        if owner != caller {
            panic!("Not the token owner");
        }
        let key = DataKey::Renter(token_id);

        let rental_info = RentalInfo {
            user: to.clone(),
            duration,
        };
        env.storage().persistent().set(&key, &rental_info);
        env.events().publish((symbol_short!("Rent"),), (token_id, to.clone(), duration));
    }
    // set user of the nft
    pub fn set_user(env: Env, caller: Address, token_id: i128, to: Address, duration: u64) {
        caller.require_auth();
        Self::__set_user(env.clone(), caller.clone(), token_id, to.clone(), duration);
    }

    // check if the user is the user of the nft
    pub fn is_user_of(env: Env, token_id: i128, user: Address) -> bool {
        let key = DataKey::Renter(token_id);
        let rental_info: Option<RentalInfo> = env.storage().persistent().get(&key);
        // check if rental info is expired
        if let Some(rental_info) = rental_info {
            let current_time = env.ledger().timestamp();
            if current_time > rental_info.duration || &rental_info.user != &user {
                return false;
            }
            return true;
        }
        false
    }

    // get user of the nft
    pub fn user_of(env: Env, token_id: i128) -> Option<Address> {
        let key = DataKey::Renter(token_id);
        let rental_info: Option<RentalInfo> = env.storage().persistent().get(&key);
        rental_info.map(|info| info.user)
    }

    /* ****************
     *  Getter functions
     *****************/
     

    // Get the token allowlist//
    pub fn is_token_id_allowed(env: Env, token_id: i128) -> bool {
        let key = DataKey::AllowedTokenId(token_id);
        env.storage()
            .persistent()
            .get::<DataKey, bool>(&key)
            .unwrap_or(false)
    }

    // Get admin address
    pub fn get_owner(env: Env) -> Address {
        env.storage()
            .persistent()
            .get::<Symbol, Address>(&ADMIN)
            .unwrap()
    }

    // Get mint price for a specific token and currency
    pub fn get_mint_price(env: Env, token_id: i128, currency: Address) -> MintPriceSettings {
        // Check if currency is native XLM
        let native_xlm = env
            .storage()
            .persistent()
            .get::<Symbol, Address>(&NATIVE)
            .unwrap();

        // Get the appropriate price settings based on currency type
        let price_settings = if currency == native_xlm {
            // For native XLM, get NatPrice
            let key = DataKey::NatPrice(token_id);
            env.storage()
                .persistent()
                .get::<DataKey, MintPriceSettings>(&key)
                .unwrap_or_else(|| MintPriceSettings {
                    enabled: false,
                    amount: 0,
                })
        } else {
            // For ERC20 tokens, get ERC20Price
            let key = DataKey::ERC20Price(token_id, currency.clone());
            env.storage()
                .persistent()
                .get::<DataKey, MintPriceSettings>(&key)
                .unwrap_or_else(|| MintPriceSettings {
                    enabled: false,
                    amount: 0,
                })
        };

        // If price is enabled, return it
        if price_settings.enabled {
            return price_settings;
        }

        // Otherwise, get default settings
        if currency == native_xlm {
            // Default native price
            return env
                .storage()
                .instance()
                .get::<Symbol, MintPriceSettings>(&DEF_NAT_PRICE)
                .unwrap_or_else(|| MintPriceSettings {
                    enabled: false,
                    amount: 0,
                });
        } else {
            // Default ERC20 price
            let def_key = DataKey::DefERC20Price(currency);
            return env
                .storage()
                .persistent()
                .get::<DataKey, MintPriceSettings>(&def_key)
                .unwrap_or_else(|| MintPriceSettings {
                    enabled: false,
                    amount: 0,
                });
        }
    }

    // write function to get the current token count
    pub fn get_token_count(env: Env) -> i128 {
        env.storage().instance().get(&TOKEN_CNT).unwrap_or(0)
    }
    /* ****************
     *  Private functions
     *****************/
    // Internal function to set default mint price
    fn _set_default_mint_price(env: &Env, currency: &Address, enabled: bool, amount: u128) {
        let settings = MintPriceSettings { enabled, amount };
        if currency
            != &env
                .storage()
                .persistent()
                .get::<Symbol, Address>(&NATIVE)
                .unwrap()
        {
            let key = DataKey::DefERC20Price(currency.clone());
            env.storage().persistent().set(&key, &settings);
        } else {
            env.storage().persistent().set(&DEF_NAT_PRICE, &settings);
        }
        env.events()
            .publish((symbol_short!("DefPrice"),), (currency, enabled, amount));
    }

    // Set the mint price for a specific token
    fn _set_mint_price(env: &Env, token_id: i128, currency: &Address, enabled: bool, amount: u128) {
        let settings = MintPriceSettings { enabled, amount };
        //
        if currency
            != &env
                .storage()
                .persistent()
                .get::<Symbol, Address>(&NATIVE)
                .unwrap()
        {
            let key = DataKey::ERC20Price(token_id, currency.clone());
            env.storage().persistent().set(&key, &settings);
        } else {
            let key = DataKey::NatPrice(token_id);
            env.storage().persistent().set(&key, &settings);
        }
        env.events()
            .publish((symbol_short!("DefPrice"),), (currency, enabled, amount));
    }

    // Set the tokens allowlist
    fn _set_tokens_allowlist(env: &Env, _apply_tokens_allowlist: bool) {
        env.storage()
            .instance()
            .set(&APPLY_ALLOW, &_apply_tokens_allowlist);
    }

    // Set the token allowlist
    fn _set_token_allowlist(env: &Env, token_id: i128, _apply_tokens_allowlist: bool) {
        let key = DataKey::AllowedTokenId(token_id);
        env.storage()
            .persistent()
            .set(&key, &_apply_tokens_allowlist);
    }

    // Internal function to mint a token
    fn _safemint(env: Env, token_id: i128, to: Address) {
        // Check if max supply is exceeded
        let token_count: i128 = env.storage().instance().get(&TOKEN_CNT).unwrap_or(0);
        let max_supply: u32 = env.storage().instance().get(&MAX_SUP).unwrap_or(0);
        assert!(
            token_id <= max_supply as i128,
            "Maximum token supply reached"
        );

        // token_id must be token_count + 1
        assert!(
            token_id == token_count + 1,
            "Token ID must be the next number"
        );

        // Update token count if this is a new token ID
        if token_id > token_count {
            env.storage().instance().set(&TOKEN_CNT, &token_id);
        }
        // Set the owner of the token
        let owner_key = DataKey::Owner(token_id);
        env.storage().persistent().set(&owner_key, &to);
        // set the user of the token
        let expiry_time = env.ledger().timestamp() + 3153600000000;  // as long as possible for the minted
        Self::__set_user(env.clone(), to.clone(), token_id, to.clone(), expiry_time);   

        env.events()
            .publish((symbol_short!("Mint"),), (to, token_id));
    }
    /* ****************
     *  Only admin functions
     *****************/
    // Set the base URI
    pub fn set_base_uri(env: Env, new_base_uri: String) {
        let admin = env
            .storage()
            .persistent()
            .get::<Symbol, Address>(&ADMIN)
            .unwrap();
        admin.require_auth();
        env.storage().instance().set(&BASE_URI, &new_base_uri);
        env.events()
            .publish((symbol_short!("BaseURI"),), (new_base_uri,));
    }

    // Set the contract URI
    pub fn set_contract_uri(env: Env, new_contract_uri: String) {
        let admin = env
            .storage()
            .persistent()
            .get::<Symbol, Address>(&ADMIN)
            .unwrap();
        admin.require_auth();
        env.storage().instance().set(&CONT_URI, &new_contract_uri);
        env.events()
            .publish((symbol_short!("ContURI"),), (new_contract_uri,));
    }

    // Set the default mint price
    pub fn set_default_mint_price(env: Env, currency: Address, enabled: bool, amount: u128) {
        let admin = env
            .storage()
            .persistent()
            .get::<Symbol, Address>(&ADMIN)
            .unwrap();
        admin.require_auth();
        Self::_set_default_mint_price(&env, &currency, enabled, amount);
    }
    // Set the mint price for a specific token
    pub fn set_mint_price(
        env: Env,
        token_id: i128,
        currency: Address,
        enabled: bool,
        amount: u128,
    ) {
        let admin = env
            .storage()
            .persistent()
            .get::<Symbol, Address>(&ADMIN)
            .unwrap();
        admin.require_auth();
        Self::_set_mint_price(&env, token_id, &currency, enabled, amount);
    }

    // Set the royalty
    pub fn set_royalty(env: Env, new_royalty_bps: u32) {
        let admin = env
            .storage()
            .persistent()
            .get::<Symbol, Address>(&ADMIN)
            .unwrap();
        admin.require_auth();
        if new_royalty_bps > 1000 {
            panic!("Royalty BPS cannot exceed 1000 (100%)");
        }
        env.storage().instance().set(&ROY_BPS, &new_royalty_bps);
        env.events()
            .publish((symbol_short!("RoyUpd"),), (new_royalty_bps,));
    }

    // Set the minter
    pub fn set_minter(env: Env, new_minter: Address) {
        let admin = env
            .storage()
            .persistent()
            .get::<Symbol, Address>(&ADMIN)
            .unwrap();
        admin.require_auth();
        env.storage().instance().set(&MINTER, &new_minter);
        env.events()
            .publish((symbol_short!("MinterUpd"),), (new_minter,));
    }

    // Set the forwarder
    pub fn set_forwarder(env: Env, new_forwarder: Address) {
        let admin = env
            .storage()
            .persistent()
            .get::<Symbol, Address>(&ADMIN)
            .unwrap();
        admin.require_auth();
        env.storage().instance().set(&FORWARD, &new_forwarder);
        env.events()
            .publish((symbol_short!("FwdUpd"),), (new_forwarder,));
    }

    // Set the tokens allowlist
    pub fn set_tokens_allowlist(env: Env, apply: bool) {
        let admin = env
            .storage()
            .persistent()
            .get::<Symbol, Address>(&ADMIN)
            .unwrap();
        admin.require_auth();
        Self::_set_tokens_allowlist(&env, apply);
        env.events().publish((symbol_short!("AllowUpd"),), (apply,));
    }

    // Set the token allowlist
    pub fn set_tokens_are_allowed(env: Env, token_ids: Vec<i128>, are_allowed: Vec<bool>) {
        let admin = env
            .storage()
            .persistent()
            .get::<Symbol, Address>(&ADMIN)
            .unwrap();
        admin.require_auth();
        // Check if the length of token_ids and are_allowed are the same
        if token_ids.len() != are_allowed.len() {
            panic!("Token IDs and are_allowed vectors must have the same length");
        }
        // Set the token allowlist for each token ID
        for i in 0..token_ids.len() {
            Self::_set_token_allowlist(
                &env,
                token_ids.get(i).unwrap(),
                are_allowed.get(i).unwrap(),
            );
        }
    }

    pub fn add_allowed_token_id(env: Env, token_id: i128) {
        let admin = env
            .storage()
            .persistent()
            .get::<Symbol, Address>(&ADMIN)
            .unwrap();
        admin.require_auth();
        let key = DataKey::AllowedTokenId(token_id);
        env.storage().persistent().set(&key, &true);
        env.events()
            .publish((symbol_short!("AllowAdd"),), (token_id,));
    }

    pub fn remove_allowed_token_id(env: Env, token_id: i128) {
        let admin = env
            .storage()
            .persistent()
            .get::<Symbol, Address>(&ADMIN)
            .unwrap();
        admin.require_auth();
        let key = DataKey::AllowedTokenId(token_id);
        env.storage().persistent().remove(&key);
        env.events()
            .publish((symbol_short!("AllowRem"),), (token_id,));
    }

    // Transfer the admin role
    pub fn transfer_admin(env: Env, new_admin: Address) {
        let admin = env
            .storage()
            .persistent()
            .get::<Symbol, Address>(&ADMIN)
            .unwrap();
        admin.require_auth();
        env.storage().persistent().set(&ADMIN, &new_admin);
        env.events()
            .publish((symbol_short!("AdminUpd"),), (new_admin,));
    }
}

mod test;

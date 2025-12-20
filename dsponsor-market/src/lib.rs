#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token, Address, Env, Map, Symbol, Vec,
};

mod dsponsor {
    // Import the on-chain dsponsor (NFT) contract client to call `transfer_from`, `owner_of`, etc.
    soroban_sdk::contractimport!(file = "../target/wasm32v1-none/release/dsponsor.wasm");
}

#[contract]
pub struct DSponsorMarket;

// Helper function to create a unique key for nft_contract + token_id
fn create_token_key(nft_contract: &Address, token_id: i128) -> (Address, i128) {
    (nft_contract.clone(), token_id)
}

// Storage keys
const ADMIN: Symbol = symbol_short!("ADMIN");
const NATIVE: Symbol = symbol_short!("NATIVE");
const FEE_BPS: Symbol = symbol_short!("FEE_BPS");
const LIST_CNT: Symbol = symbol_short!("LIST_CNT");
const AUCT_CNT: Symbol = symbol_short!("AUCT_CNT");
const LISTINGS: Symbol = symbol_short!("LISTINGS");
const AUCTIONS: Symbol = symbol_short!("AUCTIONS");
const LISTED_TOKENS: Symbol = symbol_short!("LISTED_TK");
const AUCTIONED_TOKENS: Symbol = symbol_short!("AUCT_TK");

#[contracttype]
#[derive(Clone)]
pub struct Listing {
    pub id: u32,
    pub seller: Address,
    pub nft_contract: Address,
    pub token_id: i128,
    pub currency: Address,
    pub price: u128,
    pub active: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct Auction {
    pub id: u32,
    pub seller: Address,
    pub nft_contract: Address,
    pub token_id: i128,
    pub currency: Address,
    pub reserve_price: u128,
    pub highest_bid: u128,
    pub highest_bidder: Option<Address>,
    pub active: bool,
}

#[contractimpl]
impl DSponsorMarket {
    pub fn initialize(env: &Env, admin: Address, native_xlm: Address, fee_bps: u32) {
        if env.storage().persistent().has(&ADMIN) {
            panic!("Already initialized");
        }
        if fee_bps > 1000 {
            panic!("Fee cannot exceed 100%");
        }
        env.storage().persistent().set(&ADMIN, &admin);
        env.storage().persistent().set(&NATIVE, &native_xlm);
        env.storage().persistent().set(&FEE_BPS, &fee_bps);
        env.storage().instance().set(&LIST_CNT, &0u32);
        env.storage().instance().set(&AUCT_CNT, &0u32);
        env.events().publish((symbol_short!("Init"),), (admin, native_xlm, fee_bps));
    }

    /* **********
     * Listings (fixed price)
     ********** */
    pub fn create_listing(
        env: &Env,
        seller: Address,
        nft_contract: Address,
        token_id: i128,
        currency: Address,
        price: u128,
    ) -> u32 {
        // Only owner can list
        let client = dsponsor::Client::new(env, &nft_contract);
        let owner = client.owner_of(&token_id).expect("Token not minted");
        if owner != seller {
            panic!("Only owner can list token");
        }
        seller.require_auth();

        // Check if token is already listed using efficient mapping
        let token_key = create_token_key(&nft_contract, token_id);
        let mut listed_tokens: Map<(Address, i128), bool> = env
            .storage()
            .instance()
            .get::<Symbol, Map<(Address, i128), bool>>(&LISTED_TOKENS)
            .unwrap_or(Map::new(env));
        
        if listed_tokens.get(token_key.clone()).unwrap_or(false) {
            panic!("Token is already listed. Please cancel existing listing first.");
        }

        let curr = env.storage().instance().get::<Symbol, u32>(&LIST_CNT).unwrap_or(0);
        let new_id = curr + 1;

        let mut listings: Map<u32, Listing> = env
            .storage()
            .instance()
            .get::<Symbol, Map<u32, Listing>>(&LISTINGS)
            .unwrap_or(Map::new(env));

        let listing = Listing {
            id: new_id,
            seller: seller.clone(),
            nft_contract: nft_contract.clone(),
            token_id,
            currency: currency.clone(),
            price,
            active: true,
        };
        listings.set(new_id, listing);
        env.storage().instance().set(&LISTINGS, &listings);
        env.storage().instance().set(&LIST_CNT, &new_id);

        // Mark token as listed
        listed_tokens.set(token_key, true);
        env.storage().instance().set(&LISTED_TOKENS, &listed_tokens);

        env.events().publish((symbol_short!("LIST"),), (nft_contract, token_id, price));
        new_id
    }

    pub fn cancel_listing(env: &Env, listing_id: u32, caller: Address) {
        let mut listings: Map<u32, Listing> = env
            .storage()
            .instance()
            .get::<Symbol, Map<u32, Listing>>(&LISTINGS)
            .expect("No listings");
        let mut l = listings.get(listing_id).expect("Listing not found");
        if !l.active {
            panic!("Listing not active");
        }
        if l.seller != caller {
            panic!("Only seller can cancel");
        }
        caller.require_auth();
        l.active = false;
        listings.set(listing_id, l.clone());
        env.storage().instance().set(&LISTINGS, &listings);

        // Remove token from listed tokens mapping
        let token_key = create_token_key(&l.nft_contract, l.token_id);
        let mut listed_tokens: Map<(Address, i128), bool> = env
            .storage()
            .instance()
            .get::<Symbol, Map<(Address, i128), bool>>(&LISTED_TOKENS)
            .unwrap_or(Map::new(env));
        listed_tokens.remove(token_key);
        env.storage().instance().set(&LISTED_TOKENS, &listed_tokens);

        env.events().publish((symbol_short!("LISTCXL"),), (listing_id,));
    }

    pub fn buy(env: &Env, listing_id: u32, buyer: Address) {
        let mut listings: Map<u32, Listing> = env
            .storage()
            .instance()
            .get::<Symbol, Map<u32, Listing>>(&LISTINGS)
            .expect("No listings");
        let l = listings.get(listing_id).expect("Listing not found");
        if !l.active {
            panic!("Listing not active");
        }

        // Payments
        let fee_bps = env.storage().persistent().get::<Symbol, u32>(&FEE_BPS).unwrap_or(0);
        let fee = (l.price as u128) * (fee_bps as u128) / 1000u128;
        let seller_amount = l.price - fee;
        let fee_recipient = env.storage().persistent().get::<Symbol, Address>(&ADMIN).unwrap();

        // transfer price from buyer to market
        let market = env.current_contract_address();
        buyer.require_auth();
        let native_addr = env.storage().persistent().get::<Symbol, Address>(&NATIVE).unwrap();
        let is_native = l.currency == native_addr;
        if is_native {
            let t = token::TokenClient::new(env, &native_addr);
            t.transfer(&buyer, &market, &(l.price as i128));
            // distribute funds
            t.transfer(&market, &l.seller, &(seller_amount as i128));
            if fee > 0 {
                t.transfer(&market, &fee_recipient, &(fee as i128));
            }
        } else {
            let t = token::Client::new(env, &l.currency);
            t.transfer(&buyer, &market, &(l.price as i128));
            // distribute funds
            t.transfer(&market, &l.seller, &(seller_amount as i128));
            if fee > 0 {
                t.transfer(&market, &fee_recipient, &(fee as i128));
            }
        }

        // Transfer NFT from seller to buyer via transfer_from with spender = market (requires prior approval of market by seller)
        let nft = dsponsor::Client::new(env, &l.nft_contract);
        let market_addr = env.current_contract_address();
        nft.transfer_from(&market_addr, &l.seller, &buyer, &l.token_id);

        // close listing
        let mut updated = l.clone();
        updated.active = false;
        listings.set(listing_id, updated);
        env.storage().instance().set(&LISTINGS, &listings);

        // Remove token from listed tokens mapping
        let token_key = create_token_key(&l.nft_contract, l.token_id);
        let mut listed_tokens: Map<(Address, i128), bool> = env
            .storage()
            .instance()
            .get::<Symbol, Map<(Address, i128), bool>>(&LISTED_TOKENS)
            .unwrap_or(Map::new(env));
        listed_tokens.remove(token_key);
        env.storage().instance().set(&LISTED_TOKENS, &listed_tokens);

        env.events().publish((symbol_short!("SOLD"),), (listing_id, buyer));
    }

    /* **********
     * Auctions (English)
     ********** */
    pub fn create_auction(
        env: &Env,
        seller: Address,
        nft_contract: Address,
        token_id: i128,
        currency: Address,
        reserve_price: u128,
    ) -> u32 {
        // Only owner can auction
        let client = dsponsor::Client::new(env, &nft_contract);
        let owner = client.owner_of(&token_id).expect("Token not minted");
        if owner != seller {
            panic!("Only owner can auction token");
        }
        seller.require_auth();

        // Check if token is already listed or auctioned using efficient mappings
        let token_key = create_token_key(&nft_contract, token_id);
        
        // Check if already listed
        let listed_tokens: Map<(Address, i128), bool> = env
            .storage()
            .instance()
            .get::<Symbol, Map<(Address, i128), bool>>(&LISTED_TOKENS)
            .unwrap_or(Map::new(env));
        
        if listed_tokens.get(token_key.clone()).unwrap_or(false) {
            panic!("Token is already listed. Please cancel existing listing first.");
        }
        
        // Check if already auctioned
        let auctioned_tokens: Map<(Address, i128), bool> = env
            .storage()
            .instance()
            .get::<Symbol, Map<(Address, i128), bool>>(&AUCTIONED_TOKENS)
            .unwrap_or(Map::new(env));
        
        if auctioned_tokens.get(token_key.clone()).unwrap_or(false) {
            panic!("Token is already auctioned. Please cancel existing auction first.");
        }

        let curr = env.storage().instance().get::<Symbol, u32>(&AUCT_CNT).unwrap_or(0);
        let new_id = curr + 1;

        let mut auctions: Map<u32, Auction> = env
            .storage()
            .instance()
            .get::<Symbol, Map<u32, Auction>>(&AUCTIONS)
            .unwrap_or(Map::new(env));

        let a = Auction {
            id: new_id,
            seller: seller.clone(),
            nft_contract: nft_contract.clone(),
            token_id,
            currency: currency.clone(),
            reserve_price,
            highest_bid: 0,
            highest_bidder: None,
            active: true,
        };
        auctions.set(new_id, a);
        env.storage().instance().set(&AUCTIONS, &auctions);
        env.storage().instance().set(&AUCT_CNT, &new_id);

        // Mark token as auctioned
        let mut auctioned_tokens: Map<(Address, i128), bool> = env
            .storage()
            .instance()
            .get::<Symbol, Map<(Address, i128), bool>>(&AUCTIONED_TOKENS)
            .unwrap_or(Map::new(env));
        auctioned_tokens.set(token_key, true);
        env.storage().instance().set(&AUCTIONED_TOKENS, &auctioned_tokens);

        env.events().publish((symbol_short!("AUCT"),), (nft_contract, token_id, reserve_price));
        new_id
    }

    pub fn bid(env: &Env, auction_id: u32, bidder: Address, amount: u128) {
        let mut auctions: Map<u32, Auction> = env
            .storage()
            .instance()
            .get::<Symbol, Map<u32, Auction>>(&AUCTIONS)
            .expect("No auctions");
        let mut a = auctions.get(auction_id).expect("Auction not found");
        if !a.active {
            panic!("Auction not active");
        }
        if amount <= a.highest_bid || amount < a.reserve_price {
            panic!("Bid too low");
        }
        bidder.require_auth();

        let market = env.current_contract_address();
        let native_addr = env.storage().persistent().get::<Symbol, Address>(&NATIVE).unwrap();
        let is_native = a.currency == native_addr;
        // Pull new bid into the contract escrow
        if is_native {
            let t = token::TokenClient::new(env, &native_addr);
            t.transfer(&bidder, &market, &(amount as i128));
        } else {
            let t = token::Client::new(env, &a.currency);
            t.transfer(&bidder, &market, &(amount as i128));
        }

        // Refund previous bidder (optional reward logic omitted for simplicity)
        if let Some(prev_bidder) = a.highest_bidder.clone() {
            if a.highest_bid > 0 {
                if is_native {
                    let t = token::TokenClient::new(env, &native_addr);
                    t.transfer(&market, &prev_bidder, &(a.highest_bid as i128));
                } else {
                    let t = token::Client::new(env, &a.currency);
                    t.transfer(&market, &prev_bidder, &(a.highest_bid as i128));
                }
            }
        }

        a.highest_bid = amount;
        a.highest_bidder = Some(bidder);
        auctions.set(auction_id, a);
        env.storage().instance().set(&AUCTIONS, &auctions);
        env.events().publish((symbol_short!("BID"),), (auction_id, amount));
    }

    pub fn finalize_auction(env: &Env, auction_id: u32, caller: Address) {
        // Seller must be the caller
        caller.require_auth();
        let mut auctions: Map<u32, Auction> = env
            .storage()
            .instance()
            .get::<Symbol, Map<u32, Auction>>(&AUCTIONS)
            .expect("No auctions");
        let mut a = auctions.get(auction_id).expect("Auction not found");
        if !a.active {
            panic!("Auction not active");
        }
        // Only seller can finalize
        if a.seller != caller {
            panic!("Only seller can finalize");
        }

        let market = env.current_contract_address();
        let native_addr = env.storage().persistent().get::<Symbol, Address>(&NATIVE).unwrap();
        let is_native = a.currency == native_addr;
        let fee_bps = env.storage().persistent().get::<Symbol, u32>(&FEE_BPS).unwrap_or(0);
        let fee = (a.highest_bid as u128) * (fee_bps as u128) / 1000u128;
        let seller_amount = a.highest_bid - fee;
        let fee_recipient = env.storage().persistent().get::<Symbol, Address>(&ADMIN).unwrap();

        if let Some(winner) = a.highest_bidder.clone() {
            // pay seller and fee
            if is_native {
                let t = token::TokenClient::new(env, &native_addr);
                t.transfer(&market, &a.seller, &(seller_amount as i128));
                if fee > 0 {
                    t.transfer(&market, &fee_recipient, &(fee as i128));
                }
            } else {
                let t = token::Client::new(env, &a.currency);
                t.transfer(&market, &a.seller, &(seller_amount as i128));
                if fee > 0 {
                    t.transfer(&market, &fee_recipient, &(fee as i128));
                }
            }
            // transfer NFT via transfer_from with spender = market (requires seller approval)
            let nft = dsponsor::Client::new(env, &a.nft_contract);
            let market_addr = env.current_contract_address();
            nft.transfer_from(&market_addr, &a.seller, &winner, &a.token_id);
            env.events().publish((symbol_short!("AUCT_SOLD"),), (auction_id, winner));
        } else {
            // No winner: return token to seller; no escrow to release (no bids)
            env.events().publish((symbol_short!("AUCTNOBID"),), (auction_id,));
        }

        a.active = false;
        auctions.set(auction_id, a);
        env.storage().instance().set(&AUCTIONS, &auctions);
    }

    /* **********
     * Views
     ********** */
    pub fn get_listing(env: &Env, listing_id: u32) -> Option<Listing> {
        let listings: Map<u32, Listing> = env
            .storage()
            .instance()
            .get::<Symbol, Map<u32, Listing>>(&LISTINGS)
            .unwrap_or(Map::new(env));
        listings.get(listing_id)
    }

    pub fn get_auction(env: &Env, auction_id: u32) -> Option<Auction> {
        let auctions: Map<u32, Auction> = env
            .storage()
            .instance()
            .get::<Symbol, Map<u32, Auction>>(&AUCTIONS)
            .unwrap_or(Map::new(env));
        auctions.get(auction_id)
    }

    pub fn get_all_listings(env: &Env) -> Vec<Listing> {
        let listings: Map<u32, Listing> = env
            .storage()
            .instance()
            .get::<Symbol, Map<u32, Listing>>(&LISTINGS)
            .unwrap_or(Map::new(env));
        
        let mut result = Vec::new(env);
        let mut i = 1u32;
        loop {
            if let Some(listing) = listings.get(i) {
                if listing.active {
                    result.push_back(listing);
                }
                i += 1;
            } else {
                break;
            }
        }
        result
    }

    pub fn get_all_auctions(env: &Env) -> Vec<Auction> {
        let auctions: Map<u32, Auction> = env
            .storage()
            .instance()
            .get::<Symbol, Map<u32, Auction>>(&AUCTIONS)
            .unwrap_or(Map::new(env));
        
        let mut result = Vec::new(env);
        let mut i = 1u32;
        loop {
            if let Some(auction) = auctions.get(i) {
                if auction.active {
                    result.push_back(auction);
                }
                i += 1;
            } else {
                break;
            }
        }
        result
    }
}

#[cfg(test)]
mod test;



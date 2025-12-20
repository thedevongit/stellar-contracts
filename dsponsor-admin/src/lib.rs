#![no_std]

mod test;

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token, Address, BytesN, Env, IntoVal, Map, String, Symbol, Val, Vec
};

mod dsponsor {
    soroban_sdk::contractimport!(file = "../target/wasm32v1-none/release/dsponsor.wasm");
}

mod dfactory {
    // Alias InitParams to avoid the error
    type InitParams = crate::InitParams;

    soroban_sdk::contractimport!(
        file = "../target/wasm32v1-none/release/dsponsor_factory.wasm"
    );
}

#[contract]
pub struct DSponsorAdmin;

// Structure for NFT initialization parameters
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

// Structure for offer options
#[contracttype]
#[derive(Clone)]
pub struct OfferOptions {
    pub admins: Vec<Address>,
    pub validators: Vec<Address>,
    pub ad_parameters: Vec<String>,
}

// Structure for offer initialization
#[contracttype]
#[derive(Clone)]
pub struct OfferInitParams {
    pub name: String,
    pub offer_metadata: String,
    pub options: OfferOptions,
}

// Structure for a sponsoring offer
#[contracttype]
#[derive(Clone)]
pub struct SponsoringOffer {
    pub id: u32,
    pub disabled: bool,
    pub nft_contract: Address,
    pub admins: Map<Address, bool>,
    pub validators: Map<Address, bool>,
    pub ad_parameters: Map<String, bool>,
    pub proposals: Map<u32, Map<String, SponsoringProposal>>,
    pub offer_metadata: String,
}

// Structure for a sponsoring proposal
#[contracttype]
#[derive(Clone)]
pub struct SponsoringProposal {
    pub last_validated: u32,
    pub last_rejected: u32,
    pub last_submitted: u32,
    pub data: String,
}

// Structure for mint and submission parameters
#[contracttype]
#[derive(Clone)]
pub struct MintAndSubmitParams {
    pub token_id: u32,
    pub to: Address,
    pub currency: Address,
    pub token_data: String,
    pub offer_id: u32,
    pub ad_parameters: Vec<String>,
    pub ad_datas: Vec<String>,
    pub referral_info: String,
}

// Structure for referral information
#[derive(Clone)]
#[contracttype]
pub struct ReferralInfo {
    pub enabler: Address,
    pub spender: Address,
    pub additional_info: String,
}

// Define the ReviewAdProposal struct to match the Solidity implementation
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct ReviewAdProposal {
    pub offer_id: u32,
    pub token_id: u32,
    pub proposal_id: u32,
    pub ad_parameter: String,
    pub validated: bool,
    pub reason: String,
    pub validator: Address,
}

// Structure for mint price settings
#[contracttype]
#[derive(Clone)]
pub struct MintPriceSettings {
    pub enabled: bool,
    pub amount: u128,
}

// Storage symbols
const OFFER_CNT: Symbol = symbol_short!("OFFR_CNT");
const PROP_CNT: Symbol = symbol_short!("PROP_CNT");
const OFFERS: Symbol = symbol_short!("OFFERS");
const FACTORY: Symbol = symbol_short!("FACTORY");
const BPS: Symbol = symbol_short!("BPS");
const FEE_RECIPIENT: Symbol = symbol_short!("FEE_RCPT");
const NATIVE: Symbol = symbol_short!("NATIVE");
const ADMIN: Symbol = symbol_short!("admin");

#[contractimpl]
impl DSponsorAdmin {
    // constructor
    pub fn initialize(
        env: &Env,
        nft_factory: Address,
        fee_recipient: Address,
        native_xlm: Address,
        bps: u32,
        admin: Address,
    ) {
        // Check if contract is already initialized
        if env.storage().persistent().has(&ADMIN) {
            panic!("Contract already initialized");
        }

        // Set the admin address
        env.storage().persistent().set(&FACTORY, &nft_factory);
        Self::_update_protocol_fee(env, bps, fee_recipient); // Set the protocol fee
        env.storage().persistent().set(&ADMIN, &admin);
        // Set the native currency address (XLM)
        env.storage().persistent().set(&NATIVE, &native_xlm);
        env.events().publish(
            (symbol_short!("Init"),),
            (nft_factory, native_xlm, bps, admin),
        );
    }

    /* ****************
     *  DSponsorAgreements functions
     *****************/

    // Create a new sponsoring offer
    pub fn create_offer(env: &Env, nft_contract: Address, offer_params: OfferInitParams) -> u32 {
        // Check that parameters are not empty
        if offer_params.offer_metadata.is_empty() {
            panic!("Empty offer metadata");
        }
        if offer_params.options.admins.is_empty() {
            panic!("No admins provided");
        }
        if offer_params.options.ad_parameters.is_empty() {
            panic!("No ad parameters provided");
        }

        // Increment offer counter
        let current_count = env
            .storage()
            .instance()
            .get::<Symbol, u32>(&OFFER_CNT)
            .unwrap_or(0);
        let new_offer_id = current_count + 1;
        env.storage().instance().set(&OFFER_CNT, &new_offer_id);

        // Create the new offer
        let mut offer = SponsoringOffer {
            id: new_offer_id,
            disabled: false,
            nft_contract: nft_contract.clone(),
            admins: Map::new(env),
            validators: Map::new(env),
            ad_parameters: Map::new(env),
            proposals: Map::new(env),
            offer_metadata: offer_params.offer_metadata.clone(),
        };

        // Update admins
        for admin in offer_params.options.admins.iter() {
            offer.admins.set(admin.clone(), true);
        }

        // Update validators
        for validator in offer_params.options.validators.iter() {
            offer.validators.set(validator.clone(), true);
        }

        // Update ad parameters
        for param in offer_params.options.ad_parameters.iter() {
            offer.ad_parameters.set(param.clone(), true);
        }

        // Save the offer
        let mut offers: Map<u32, SponsoringOffer> = env
            .storage()
            .instance()
            .get::<Symbol, Map<u32, SponsoringOffer>>(&OFFERS)
            .unwrap_or(Map::new(env));
        offers.set(new_offer_id, offer);
        env.storage().instance().set(&OFFERS, &offers);
        env.events()
        .publish((symbol_short!("OFFER"),), (nft_contract, new_offer_id));
        new_offer_id
    }

    // private _submit_ad_proposal function
    fn __submit_ad_proposal(
        env: &Env,
        caller: Address,
        offer_id: u32,
        token_id: u32,
        ad_parameter: String,
        data: String,
    ) -> u32 {
        // Get all offers
        let mut offers: Map<u32, SponsoringOffer> = env
            .storage()
            .instance()
            .get::<Symbol, Map<u32, SponsoringOffer>>(&OFFERS)
            .expect("No offers found");

        // Get the specific offer
        let mut offer = offers.get(offer_id).expect("Offer does not exist");
        // Only the sponsor can submit an ad proposal
        let client = dsponsor::Client::new(&env, &offer.nft_contract);
        let is_user = client.is_user_of(&(token_id as i128), &caller);
        if !is_user {
            panic!("Only the sponsor can submit an ad proposal");
        }
        if offer.disabled {
            panic!("Offer is disabled");
        }

        // Check that the ad parameter is allowed
        if !offer
            .ad_parameters
            .get(ad_parameter.clone())
            .unwrap_or(false)
        {
            panic!("Unallowed ad parameter");
        }

        // Check that data is not empty
        if data.is_empty() {
            panic!("No ad data submitted");
        }

        // Increment proposal counter
        let current_count = env
            .storage()
            .instance()
            .get::<Symbol, u32>(&PROP_CNT)
            .unwrap_or(0);
        let new_proposal_id = current_count + 1;
        env.storage().instance().set(&PROP_CNT, &new_proposal_id);

        // Update the proposal
        let mut proposals = offer.proposals.get(token_id).unwrap_or(Map::new(env));
        let mut proposal = proposals
            .get(ad_parameter.clone())
            .unwrap_or(SponsoringProposal {
                last_validated: 0,
                last_rejected: 0,
                last_submitted: 0,
                data: String::from_str(&env, ""),
            });
        proposal.last_submitted = new_proposal_id;
        proposal.data = data;
        proposals.set(ad_parameter.clone(), proposal);
        offer.proposals.set(token_id, proposals);

        // Update the offer in the map
        offers.set(offer_id, offer.clone());

        // Save changes
        env.storage().instance().set(&OFFERS, &offers);
        env.events().publish(
            (symbol_short!("PROPOSAL"),),
            (
                offer.nft_contract,
                &token_id,
                &caller,
                "DirectSubmission",
                0,
            ),
        );
        // Return the proposal ID
        new_proposal_id
    }
    // Submit an ad proposal
    pub fn submit_ad_proposal(
        env: &Env,
        caller: Address,
        offer_id: u32,
        token_id: u32,
        ad_parameter: String,
        data: String,
    ) -> u32 {
        caller.require_auth();
        Self::__submit_ad_proposal(env, caller, offer_id, token_id, ad_parameter, data)
    }

    // Submit multiple ad proposals
    pub fn submit_ad_proposals(
        env: &Env,
        offer_ids: Vec<u32>,
        token_ids: Vec<u32>,
        ad_parameters: Vec<String>,
        datas: Vec<String>,
        caller: Address,
    ) {
        caller.require_auth();
        if offer_ids.len() != token_ids.len()
            || offer_ids.len() != ad_parameters.len()
            || offer_ids.len() != datas.len()
        {
            panic!("Parameters and data arrays must have the same length");
        }
        for i in 0..offer_ids.len() {
            Self::__submit_ad_proposal(
                env,
                caller.clone(),
                offer_ids.get(i).unwrap(),
                token_ids.get(i).unwrap(),
                ad_parameters.get(i).unwrap(),
                datas.get(i).unwrap(),
            );
        }
    }

    // Review an ad proposal (approve or reject)
    pub fn review_ad_proposal(
        env: &Env,
        offer_id: u32,
        token_id: u32,
        proposal_id: u32,
        ad_parameter: String,
        validated: bool,
        reason: String,
        validator: Address,
    ) -> u32 {
        // Get all offers
        let mut offers: Map<u32, SponsoringOffer> = env
            .storage()
            .instance()
            .get::<Symbol, Map<u32, SponsoringOffer>>(&OFFERS)
            .expect("No offers found");

        // Get the specific offer
        let mut offer = offers.get(offer_id).expect("Offer does not exist");

        if offer.disabled {
            panic!("Offer is disabled");
        }

        // Check that the validator is authorized
        if !offer.validators.get(validator.clone()).unwrap_or(false) {
            panic!("Unauthorized validator");
        }

        // Verify that the validator is the caller
        validator.require_auth();

        // Check that the ad parameter is allowed
        if !offer
            .ad_parameters
            .get(ad_parameter.clone())
            .unwrap_or(false)
        {
            panic!("Unallowed ad parameter");
        }

        // Get proposals for this token
        let mut proposals = offer.proposals.get(token_id).unwrap_or(Map::new(env));

        // Get the specific proposal
        let mut proposal = proposals
            .get(ad_parameter.clone())
            .unwrap_or(SponsoringProposal {
                last_validated: 0,
                last_rejected: 0,
                last_submitted: 0,
                data: String::from_str(&env, ""),
            });

        // Check that the proposal ID matches the last submitted proposal
        if proposal_id != proposal.last_submitted {
            panic!("Proposal not submitted by sponsor");
        }

        // Update the proposal based on approval
        if validated {
            proposal.last_validated = proposal_id;
        } else {
            proposal.last_rejected = proposal_id;
        }

        // The validator action is final. He cannot change the status of the proposal
        proposal.last_submitted = 0;

        // Save changes
        proposals.set(ad_parameter.clone(), proposal);
        offer.proposals.set(token_id, proposals);
        offers.set(offer_id, offer);
        env.storage().instance().set(&OFFERS, &offers);

        // Emit event
        env.events().publish(
            ("REVIEW",),
            (
                offer_id,
                token_id,
                proposal_id,
                ad_parameter,
                validated,
                reason,
            ),
        );

        // Return the proposal ID
        proposal_id
    }

    // Review multiple ad proposals (approve or reject)
    pub fn review_ad_proposals(env: &Env, reviews: Vec<ReviewAdProposal>) -> Vec<u32> {
        // Check that the vector is not empty
        if reviews.is_empty() {
            panic!("No reviews provided");
        }

        let mut review_ids = Vec::new(env);

        // Process each review
        for review in reviews.iter() {
            let review_id = Self::review_ad_proposal(
                env,
                review.offer_id,
                review.token_id,
                review.proposal_id,
                review.ad_parameter.clone(),
                review.validated,
                review.reason.clone(),
                review.validator.clone(),
            );
            review_ids.push_back(review_id);
        }

        review_ids
    }

    // Update an existing offer
    pub fn update_offer(
        env: &Env,
        offer_id: u32,
        admin: Address,
        offer_params: OfferInitParams,
    ) -> bool {
        // Check that parameters are not empty
        if offer_params.offer_metadata.is_empty() {
            panic!("Empty offer metadata");
        }
        if offer_params.options.admins.is_empty() {
            panic!("No admins provided");
        }
        if offer_params.options.ad_parameters.is_empty() {
            panic!("No ad parameters provided");
        }

        // Get all offers
        let mut offers: Map<u32, SponsoringOffer> = env
            .storage()
            .instance()
            .get::<Symbol, Map<u32, SponsoringOffer>>(&OFFERS)
            .expect("No offers found");

        // Get the specific offer
        let mut offer = offers.get(offer_id).expect("Offer does not exist");

        // Check that the admin is authorized
        if !offer.admins.get(admin.clone()).unwrap_or(false) {
            panic!("Unauthorized admin");
        }

        // Reset maps
        offer.admins = Map::new(env);
        offer.validators = Map::new(env);
        offer.ad_parameters = Map::new(env);

        // Update admins
        for admin_addr in offer_params.options.admins.iter() {
            offer.admins.set(admin_addr.clone(), true);
        }

        // Update validators
        for validator in offer_params.options.validators.iter() {
            offer.validators.set(validator.clone(), true);
        }

        // Update ad parameters
        for param in offer_params.options.ad_parameters.iter() {
            offer.ad_parameters.set(param.clone(), true);
        }

        // Update the offer in the map
        offers.set(offer_id, offer);

        // Save changes
        env.storage().instance().set(&OFFERS, &offers);

        // Return true to indicate successful update
        true
    }

    // Get the NFT contract associated with an offer
    pub fn get_offer_contract(env: &Env, offer_id: u32) -> Address {
        // Get all offers
        let offers: Map<u32, SponsoringOffer> = env
            .storage()
            .instance()
            .get::<Symbol, Map<u32, SponsoringOffer>>(&OFFERS)
            .expect("No offers found");

        // Get the specific offer
        let offer = offers.get(offer_id).expect("Offer does not exist");

        // Return the NFT contract address
        offer.nft_contract
    }

    // Get the total number of offers
    pub fn get_total_offers(env: &Env) -> u32 {
        // Get all offers
        let offers: Map<u32, SponsoringOffer> = env
            .storage()
            .instance()
            .get::<Symbol, Map<u32, SponsoringOffer>>(&OFFERS)
            .expect("No offers found");

        // Return the total number of offers
        offers.len()
    }

    // Get all offers
    pub fn get_all_offers(env: &Env) -> Vec<SponsoringOffer> {
        let offers: Map<u32, SponsoringOffer> = env
            .storage()
            .instance()
            .get::<Symbol, Map<u32, SponsoringOffer>>(&OFFERS)
            .expect("No offers found");
        // Return the offers
        offers.values()
    }
    
    // Get user offers
    pub fn get_user_offers(env: &Env, user: Address) -> Vec<SponsoringOffer> {
        let offers: Map<u32, SponsoringOffer> = env
            .storage()
            .instance()
            .get::<Symbol, Map<u32, SponsoringOffer>>(&OFFERS)
            .expect("No offers found");
        // Get the offers for the user
        let mut user_offers = Vec::new(env);
        for offer in offers.values() {
            if offer.admins.get(user.clone()).unwrap_or(false) {
                user_offers.push_back(offer.clone());
            }
        }
        user_offers
    }
    
    // Get proposals for a specific token in an offer
    pub fn get_offer_proposals(
        env: &Env,
        offer_id: u32,
        token_id: u32,
    ) -> Map<String, SponsoringProposal> {
        // Get all offers
        let offers: Map<u32, SponsoringOffer> = env
            .storage()
            .instance()
            .get::<Symbol, Map<u32, SponsoringOffer>>(&OFFERS)
            .expect("No offers found");

        // Get the specific offer
        let offer = offers.get(offer_id).expect("Offer does not exist");

        // Get proposals for this token
        offer.proposals.get(token_id).unwrap_or(Map::new(env))
    }

    // Check if an ad parameter is allowed for an offer
    pub fn is_allowed_ad_parameter(env: &Env, offer_id: u32, ad_parameter: String) -> bool {
        // Get all offers
        let offers: Map<u32, SponsoringOffer> = env
            .storage()
            .instance()
            .get::<Symbol, Map<u32, SponsoringOffer>>(&OFFERS)
            .expect("No offers found");

        // Get the specific offer
        let offer = offers.get(offer_id).expect("Offer does not exist");

        // Check if the ad parameter is allowed
        offer.ad_parameters.get(ad_parameter).unwrap_or(false)
    }

    // Check if an address is an admin of an offer
    pub fn is_offer_admin(env: &Env, offer_id: u32, admin: Address) -> bool {
        // Get all offers
        let offers: Map<u32, SponsoringOffer> = env
            .storage()
            .instance()
            .get::<Symbol, Map<u32, SponsoringOffer>>(&OFFERS)
            .expect("No offers found");

        // Get the specific offer
        let offer = offers.get(offer_id).expect("Offer does not exist");

        // Check if the address is an admin
        offer.admins.get(admin).unwrap_or(false)
    }

    // Check if an offer is disabled
    pub fn is_offer_disabled(env: &Env, offer_id: u32) -> bool {
        // Get all offers
        let offers: Map<u32, SponsoringOffer> = env
            .storage()
            .instance()
            .get::<Symbol, Map<u32, SponsoringOffer>>(&OFFERS)
            .expect("No offers found");

        // Get the specific offer
        let offer = offers.get(offer_id).expect("Offer does not exist");

        // Return the disabled state of the offer
        offer.disabled
    }

    // Check if an address is a validator of an offer
    pub fn is_offer_validator(env: &Env, offer_id: u32, validator: Address) -> bool {
        // Get all offers
        let offers: Map<u32, SponsoringOffer> = env
            .storage()
            .instance()
            .get::<Symbol, Map<u32, SponsoringOffer>>(&OFFERS)
            .expect("No offers found");

        // Get the specific offer
        let offer = offers.get(offer_id).expect("Offer does not exist");

        // Check if the address is a validator
        offer.validators.get(validator).unwrap_or(false)
    }

    /* ****************
     *  ProtocolFee functions
     *****************/

    // Internal function to update protocol fee
    fn _update_protocol_fee(env: &Env, new_bps: u32, new_recipient: Address) {
        // Check that the new fee is valid (between 0 and 1000)
        if new_bps > 1000 {
            panic!("Protocol fee cannot exceed 100%");
        }

        // Update the protocol fee and recipient
        env.storage().persistent().set(&BPS, &new_bps);
        env.storage()
            .persistent()
            .set(&FEE_RECIPIENT, &new_recipient);
    }

    fn _pay(env: &Env, from: Address, to: Address, amount: u128, currency: Address) {
        // Only proceed if amount is greater than 0
        if amount > 0 {
            // Get the current contract address
            let current_contract_address = env.current_contract_address();

            // Convert amount to i128 for the token transfer
            let amount_i128: i128 = amount as i128;
            // Check if currency is the zero address (native currency)
            let native_address: Address = env
                .storage()
                .persistent()
                .get::<Symbol, Address>(&NATIVE)
                .expect("Native currency address not set");
            let is_native_currency: bool = currency == native_address;
            if is_native_currency {
                // Handle native currency (XLM)
                if from != current_contract_address {
                    panic!("Cannot send value from sender");
                } else {
                    let token_client = token::TokenClient::new(env, &native_address);
                    token_client.transfer(&from, &to, &amount_i128);
                }
            } else {
                // Handle ERC20 tokens
                let token_client = token::Client::new(env, &currency);

                if from == current_contract_address {
                    // If sending from the contract itself
                    token_client.transfer(&current_contract_address, &to, &amount_i128);
                } else {
                    // If sending from another address
                    token_client.transfer_from(&current_contract_address, &from, &to, &amount_i128);
                }
            }

            env.events()
                .publish((symbol_short!("PAYMENT"),), (from, to, amount, currency));
        }
    }

    // Internal function to pay a fee
    fn _pay_fee(
        env: &Env,
        from: Address,
        currency: Address,
        fee_amount: u128,
        origin: Address,
        referral_info: String,
    ) {
        // Get the fee recipient address
        let fee_recipient = env
            .storage()
            .persistent()
            .get::<Symbol, Address>(&FEE_RECIPIENT)
            .expect("Fee recipient not set");

        // Clone currency before passing to _pay
        let currency_clone = currency.clone();

        // Pay the fee to the recipient
        Self::_pay(env, from, fee_recipient, fee_amount, currency);

        env.events().publish(
            (symbol_short!("FeePaid"),),
            (origin, currency_clone, fee_amount, referral_info),
        );
    }

    // Calculate the fee amount based on the base amount and protocol fee percentage
    pub fn get_fee_amount(env: &Env, base_amount: u128) -> u128 {
        // Get the protocol fee percentage in basis points
        let bps = env
            .storage()
            .persistent()
            .get::<Symbol, u32>(&BPS)
            .unwrap_or(0);

        // Calculate fee amount: (base_amount * bps) / 1000
        // Using checked arithmetic to prevent overflow
        let fee_amount = (base_amount as u128)
            .checked_mul(bps as u128)
            .unwrap_or(0)
            .checked_div(1000)
            .unwrap_or(0);

        fee_amount
    }

    // Handle external calls with protocol fee handling
    fn _external_call_with_protocol_fee(
        env: &Env,
        target: Address,
        currency: Address,
        base_amount: u128,
        referral_info: ReferralInfo,
        params: MintAndSubmitParams,
    ) -> bool {
        // Clone target address at the start
        let target_addr = target.clone();

        // Calculate the fee amount
        let fee_amount = Self::get_fee_amount(env, base_amount);
        let total_amount = base_amount + fee_amount;

        // Get the current contract address
        let current_contract_address = env.current_contract_address();

        // no need to check if currency is native currency, because in soroban xlm can be handled like a token
        // For ERC20 tokens, we need to transfer the total amount from the sender to this contract
        let token_client = token::Client::new(env, &currency);
        // Clone the addresses to avoid move issues
        let contract_address_clone = current_contract_address.clone();
        let target_clone = target_addr.clone();

        let native_address: Address = env
            .storage()
            .persistent()
            .get::<Symbol, Address>(&NATIVE)
            .expect("Native currency address not set");
        let is_native_currency: bool = currency == native_address;
        if is_native_currency {
            // For native XLM, we need to ensure the spender has authorized the transfer
            referral_info.spender.require_auth();
            // Transfer the total amount from the sender to this contract
            token_client.transfer(
                &referral_info.spender,
                &current_contract_address,
                &(total_amount as i128),
            );
        } else {
            // Transfer the total amount from the sender to this contract
            token_client.transfer_from(
                &contract_address_clone,
                &referral_info.spender,
                &current_contract_address,
                &(total_amount as i128),
            );
        }
        // Clone again for approve
        // Approve the target to spend the base amount
        token_client.approve(
            &contract_address_clone,
            &target_clone,
            &(base_amount as i128),
            &(env.ledger().sequence() + 100),
        );
        // Pay the fee to the fee recipient
        Self::_pay_fee(
            env,
            current_contract_address.clone(),
            currency,
            fee_amount,
            target_addr.clone(),
            referral_info.additional_info,
        );
        // Execute the external call to the target contract
        let client = dsponsor::Client::new(&env, &target_addr);
        client.mint(
            &current_contract_address,
            &(params.token_id as i128),
            &params.to,
            &params.currency,
        );
        true
    }

    /* ****************
     *  DSponsor admin main functions
     *****************/

    // Create a new DSponsor NFT contract HERE instead of using the factory due to soroban limitations
    // Error trying to access contract instance outside of the footprint when call the factory or this function in another function
     pub fn create_dsponsor_nft(
        env: Env,
        init_params: InitParams,
        salt: Option<BytesN<32>>,
    ) -> Address {
        let native_xlm = env
            .storage()
            .persistent()
            .get::<Symbol, Address>(&NATIVE)
            .expect("Native token address not set");
        let mut init_params = init_params;
        init_params.minter = env.current_contract_address();
        let constructor_args: Vec<Val> = (init_params, native_xlm).into_val(&env);
        // Use provided salt or generate a random one
        let salt_bytes = match salt {
            Some(s) => s.to_array(),
            None => {
                // Generate a random salt
                let mut random_bytes = [0u8; 32];
                for i in 0..32 {
                    // Use a combination of timestamp and position to create randomness
                    random_bytes[i] = (env.ledger().timestamp() as u8).wrapping_add(i as u8);
                }
                random_bytes
            }
        };

        let salt = BytesN::from_array(&env, &salt_bytes);

        // Upload the contract's Wasm
        let wasm_hash = env.deployer().upload_contract_wasm(dsponsor::WASM);

        // Deploy the contract
        let contract_id = env
            .deployer()
            .with_address(env.current_contract_address(), salt)
            .deploy_v2(wasm_hash, constructor_args);
        env.events()
            .publish((symbol_short!("NewSponso"),), (&contract_id,));
        contract_id
    }
    /// Updates the protocol fee settings
    ///
    /// # Arguments
    /// * `new_bps` - The new fee in basis points (1 basis point = 0.01%)
    /// * `new_recipient` - The new address that will receive the protocol fees
    ///
    /// # Returns
    /// Returns true if the update was successful
    pub fn update_protocol_fee(env: &Env, new_bps: u32, new_recipient: Address) -> bool {
        let admin = env
            .storage()
            .persistent()
            .get::<Symbol, Address>(&ADMIN)
            .unwrap();
        admin.require_auth();
        // Clone the recipient address
        let recipient_clone = new_recipient.clone();

        // Update the protocol fee
        Self::_update_protocol_fee(env, new_bps, new_recipient);

        // Log the update
        env.events()
            .publish((symbol_short!("FeeUpdate"),), (new_bps, recipient_clone));
        true
    }

    /// Creates a new DSponsor NFT and an associated offer in a single transaction
    ///
    /// # Arguments
    /// * `init_params` - Parameters for initializing the NFT contract
    /// * `offer_params` - Parameters for creating the offer
    ///
    /// # Returns
    /// Returns the offer ID if successful
    pub fn create_dsponsor_nft_and_offer(
        env: &Env,
        init_params: InitParams,
        offer_params: OfferInitParams,
    ) -> u32 {
        // Get the factory address
        let factory = env
            .storage()
            .persistent()
            .get::<Symbol, Address>(&FACTORY)
            .expect("Factory address not set");
        let native_xlm = env
            .storage()
            .persistent()
            .get::<Symbol, Address>(&NATIVE)
            .expect("Native token address not set");
        // Force the init_params minter to be the admin by creating a mutable copy
        let mut init_params = init_params;
        init_params.minter = env.current_contract_address();
        // Create the NFT contract
        let nft_contract = dfactory::Client::new(env, &factory).create_dsponsor_nft(
            &init_params,
            &native_xlm,
            &None,
        );

        // Create the offer
        let offer_id = Self::create_offer(env, nft_contract.clone(), offer_params);

        env.events()
            .publish((symbol_short!("OFFER"),), (nft_contract, offer_id));
        offer_id
    }

    /// Mints an NFT and submits ad proposals in a single transaction
    ///
    /// # Arguments
    /// * `params` - Parameters for minting and submitting ad proposals
    ///
    /// # Returns
    /// Returns true if successful
    pub fn mint_and_submit(env: &Env, params: MintAndSubmitParams) -> bool {
        // Get the offer contract
        let offer_id = params.offer_id;
        let nft_contract = Self::get_offer_contract(env, offer_id);
        let nft_contract_clone = nft_contract.clone();
        let currency_clone = params.currency.clone();
        let params_clone = params.clone();

        // Create referral info
        let referral_info = ReferralInfo {
            enabler: env.current_contract_address(),
            spender: params.to.clone(),
            additional_info: params.referral_info,
        };

        // // Get the mint price
        let client = dsponsor::Client::new(env, &nft_contract);
        let price_settings = client.get_mint_price(&(params.token_id as i128), &params.currency);
        // Check if price is enabled
        if !price_settings.enabled {
            panic!("Price not enabled for this token and currency");
        }

        // Execute the external call with protocol fee handling
        Self::_external_call_with_protocol_fee(
            env,
            nft_contract,
            params.currency,
            price_settings.amount,
            referral_info,
            params_clone,
        );

        // Submit ad proposals
        if !params.ad_parameters.is_empty() {
            for i in 0..params.ad_parameters.len() {
                Self::__submit_ad_proposal(
                    env,
                    params.to.clone(),
                    offer_id,
                    params.token_id,
                    params.ad_parameters.get(i).unwrap(),
                    params.ad_datas.get(i).unwrap(),
                );
            }
        }

        env.events().publish(
            (symbol_short!("PROPOSAL"),),
            (
                nft_contract_clone,
                params.token_id,
                params.to,
                currency_clone,
                price_settings.amount,
            ),
        );

        true
    }
}

#![no_std]

use dsponsor::InitParams;
/// This contract demonstrates the 'factory' pattern for programmatically
/// deploying the contracts via `env.deployer()`.
use soroban_sdk::{contract, contractimpl, symbol_short, Address, BytesN, Env, IntoVal, Val, Vec};

mod dsponsor {
    soroban_sdk::contractimport!(file = "../target/wasm32v1-none/release/dsponsor.wasm");
}

#[contract]
pub struct DSponsorFactory;

#[contractimpl]
impl DSponsorFactory {
    /// Deploys the contract on behalf of the `Deployer` contract.
    ///
    /// This has to be authorized by the `Deployer`s administrator.    
    pub fn create_dsponsor_nft(
        env: Env,
        init_params: InitParams,
        native_xlm: Address,
        salt: Option<BytesN<32>>,
    ) -> Address {
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
}

mod test;

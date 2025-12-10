use alloy::{
    primitives::{Address, B256, U256},
    providers::ext::AnvilApi
};
use evm_knowledge::{
    contract_bindings::gate_lock::GateLock,
    environment_deployment::{deploy_lock_contract, spin_up_anvil_instance, AnvilControls},
    fetch_values
};
use revm::{primitives::keccak256, DatabaseRef};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let controls = spin_up_anvil_instance().await?;
    let payload = fetch_values();

    let deploy_address = deploy_lock_contract(&controls, payload).await?;

    assert!(solve(deploy_address, controls).await?);
    Ok(())
}

// Signature narrowed to AnvilControls so we can reach the provider and use
// the Anvil admin RPC to write storage; DatabaseRef alone is read-only.
async fn solve(contract_address: Address, controls: AnvilControls) -> eyre::Result<bool> {
    let provider = controls.provider.clone();
    let value_map_slot = U256::from(2);
    let is_unlocked_mask = U256::from(1) << 224;

    // Read the total length from the storage slot 4
    let total_len_word = controls.storage_ref(contract_address, U256::from(4))?;
    let total_len = total_len_word.as_limbs()[0] as usize;

    let mut ids: Vec<U256> = Vec::with_capacity(total_len);
    let mut cur_key = U256::ZERO;

    for _ in 0..total_len {
        ids.push(cur_key);

        // Compute mapping slot: keccak256(key, slot)
        let slot = compute_mapping_slot(cur_key, value_map_slot);

        // Load current packed struct and flip the is_unlocked flag
        let packed = controls.storage_ref(contract_address, slot)?;
        let updated: U256 = packed | is_unlocked_mask;

        // Write the modified word back into anvil's state
        provider
            .anvil_set_storage_at(contract_address, slot, B256::from(updated.to_be_bytes::<32>()))
            .await?;

        // Unpack fields to walk the slot chain: uint64 firstValue, uint160 secondValue
        let first: U256 = packed & ((U256::from(1u128) << 64) - U256::from(1));
        let second: U256 = (packed >> 64) & ((U256::from(1u128) << 160) - U256::from(1));

        let is_first_even = first.as_limbs()[0] % 2 == 0;
        cur_key = if is_first_even { first } else { second };
    }

    // Verify against the contract view
    let contract = GateLock::new(contract_address, provider);
    let res = contract.isSolved(ids).call().await?.res;
    Ok(res)
}

fn compute_mapping_slot(key: U256, slot: U256) -> U256 {
    let mut buf = [0u8; 64];
    buf[..32].copy_from_slice(&key.to_be_bytes::<32>());
    buf[32..].copy_from_slice(&slot.to_be_bytes::<32>());

    let hashed = keccak256(buf);
    U256::from_be_bytes(hashed.0)
}

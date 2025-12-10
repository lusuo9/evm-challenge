use std::{env, fs, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Locate the GateLock artifact produced by forge.
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
    let abi_path = Path::new(&manifest_dir).join("contracts/out/GateLock.sol/GateLock.json");

    // Ask Cargo to rerun the build script when the ABI or Solidity file changes.
    println!("cargo:rerun-if-changed={}", abi_path.display());
    println!(
        "cargo:rerun-if-changed={}",
        Path::new(&manifest_dir)
            .join("contracts/src/GateLock.sol")
            .display()
    );

    // Emit the bindings module into OUT_DIR; src/contract_bindings.rs will include!
    // it.
    let out_dir = env::var("OUT_DIR")?;
    let bindings_path = Path::new(&out_dir).join("contract_bindings.rs");
    let abi_literal = "contracts/out/GateLock.sol/GateLock.json";

    let contents = format!(
        r#"#[rustfmt::skip]
pub mod gate_lock {{
    alloy::sol!(
        #[allow(missing_docs)]
        #[sol(rpc, abi)]
        #[derive(Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        GateLock,
        "{abi_path}"
    );
}}
"#,
        abi_path = abi_literal
    );

    fs::write(bindings_path, contents)?;
    Ok(())
}

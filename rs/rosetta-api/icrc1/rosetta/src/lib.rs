use anyhow::{bail, Context};
use common::storage::{storage_client::StorageClient, types::MetadataEntry};
use ic_base_types::CanisterId;
use icrc_ledger_types::icrc::generic_metadata_value::MetadataValue;
use num_traits::ToPrimitive;
use std::{collections::HashMap, sync::Arc};

pub mod common;

pub mod ledger_blocks_synchronization;

pub struct AppState {
    pub ledger_id: CanisterId,
    pub storage: Arc<StorageClient>,
    pub metadata: Metadata,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Metadata {
    pub symbol: String,
    pub decimals: u8,
}

impl Metadata {
    const METADATA_DECIMALS_KEY: &str = "icrc1:decimals";
    const METADATA_SYMBOL_KEY: &str = "icrc1:symbol";

    pub fn from_args(symbol: String, decimals: u8) -> Self {
        Self { symbol, decimals }
    }

    pub fn from_metadata_entries(entries: &[MetadataEntry]) -> anyhow::Result<Self> {
        let entries = entries
            .iter()
            .map(|entry| {
                let value = entry.value()?;
                Ok((entry.key.clone(), value))
            })
            .collect::<anyhow::Result<HashMap<_, _>>>()?;

        let decimals = entries
            .get(Self::METADATA_DECIMALS_KEY)
            .context("Could not find decimals in metadata entries.")
            .map(|value| match value {
                MetadataValue::Nat(decimals) => decimals
                    .0
                    .to_u8()
                    .context("Decimals cannot fit into an u8."),
                _ => bail!("Could not extract decimals from metadata."),
            })??;

        let symbol = entries
            .get(Self::METADATA_SYMBOL_KEY)
            .context("Could not find symbol in metadata entries.")
            .map(|value| match value {
                MetadataValue::Text(symbol) => Ok(symbol.clone()),
                _ => bail!("Could not extract symbol from metadata."),
            })??;

        Ok(Self { symbol, decimals })
    }
}

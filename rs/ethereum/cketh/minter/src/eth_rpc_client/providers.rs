use candid::CandidType;
use ic_cdk::api::management_canister::http_request::HttpHeader;
use serde::{Deserialize, Serialize};

pub(crate) const MAINNET_PROVIDERS: &[RpcService] = &[
    RpcService::EthMainnet(EthMainnetService::Alchemy),
    RpcService::EthMainnet(EthMainnetService::Ankr),
    RpcService::EthMainnet(EthMainnetService::PublicNode),
    RpcService::EthMainnet(EthMainnetService::Cloudflare),
];

pub(crate) const SEPOLIA_PROVIDERS: &[RpcService] = &[
    RpcService::EthSepolia(EthSepoliaService::Alchemy),
    RpcService::EthSepolia(EthSepoliaService::Ankr),
    RpcService::EthSepolia(EthSepoliaService::BlockPi),
    RpcService::EthSepolia(EthSepoliaService::PublicNode),
];

// Default RPC services for unknown EVM network
pub(crate) const UNKNOWN_PROVIDERS: &[RpcService] = &[];

#[derive(Clone, PartialEq, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize, CandidType)]
pub struct RpcApi {
    pub url: String,
    pub headers: Option<Vec<HttpHeader>>,
}

#[derive(Clone, PartialEq, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize, CandidType)]
pub enum RpcService {
    EthMainnet(EthMainnetService),
    EthSepolia(EthSepoliaService),
    Chain(u64),
    Provider(u64),
    Custom(RpcApi),
}

impl std::fmt::Debug for RpcService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpcService::EthMainnet(service) => write!(f, "{:?}", service),
            RpcService::EthSepolia(service) => write!(f, "{:?}", service),
            RpcService::Chain(chain_id) => write!(f, "Chain({})", chain_id),
            RpcService::Provider(provider_id) => write!(f, "Provider({})", provider_id),
            RpcService::Custom(_) => write!(f, "Custom(..)"), // redact credentials
        }
    }
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize, CandidType,
)]
pub enum EthMainnetService {
    Alchemy,
    Ankr,
    BlockPi,
    PublicNode,
    Cloudflare,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize, CandidType,
)]
pub enum EthSepoliaService {
    Alchemy,
    Ankr,
    BlockPi,
    PublicNode,
}

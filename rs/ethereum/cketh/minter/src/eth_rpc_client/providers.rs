use candid::CandidType;
use ic_cdk::api::management_canister::http_request::HttpHeader;
use serde::Deserialize;

pub(crate) const MAINNET_PROVIDERS: &[RpcNodeProvider] = &[
    RpcNodeProvider::Ethereum(EthereumProvider::Ankr),
    RpcNodeProvider::Ethereum(EthereumProvider::PublicNode),
    RpcNodeProvider::Ethereum(EthereumProvider::Cloudflare),
];

pub(crate) const SEPOLIA_PROVIDERS: &[RpcNodeProvider] = &[
    RpcNodeProvider::Sepolia(SepoliaProvider::Ankr),
    RpcNodeProvider::Sepolia(SepoliaProvider::PublicNode),
];

#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash, Deserialize, CandidType)]
pub struct RpcApi {
    pub url: String,
    pub headers: Vec<HttpHeader>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Hash, Deserialize, CandidType)]
pub enum RpcNodeProvider {
    Ethereum(EthereumProvider),
    Sepolia(SepoliaProvider),
}

impl RpcNodeProvider {
    pub fn api(&self) -> RpcApi {
        let url = match self {
            Self::Ethereum(provider) => provider.ethereum_mainnet_endpoint_url(),
            Self::Sepolia(provider) => provider.ethereum_sepolia_endpoint_url(),
        }
        .to_string();
        RpcApi {
            url,
            headers: vec![],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Hash, Deserialize, CandidType)]
pub enum EthereumProvider {
    //https://www.ankr.com/rpc/
    Ankr,
    // https://publicnode.com/
    PublicNode,
    // https://developers.cloudflare.com/web3/ethereum-gateway/
    Cloudflare,
}

impl EthereumProvider {
    fn ethereum_mainnet_endpoint_url(&self) -> &str {
        match self {
            EthereumProvider::Ankr => "https://rpc.ankr.com/eth",
            EthereumProvider::PublicNode => "https://ethereum.publicnode.com",
            EthereumProvider::Cloudflare => "https://cloudflare-eth.com",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Hash, Deserialize, CandidType)]
pub enum SepoliaProvider {
    //https://www.ankr.com/rpc/
    Ankr,
    // https://publicnode.com/
    PublicNode,
}

impl SepoliaProvider {
    fn ethereum_sepolia_endpoint_url(&self) -> &str {
        match self {
            SepoliaProvider::Ankr => "https://rpc.ankr.com/eth_sepolia",
            SepoliaProvider::PublicNode => "https://ethereum-sepolia.publicnode.com",
        }
    }
}

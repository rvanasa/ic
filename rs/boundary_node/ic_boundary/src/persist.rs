use std::sync::Arc;

use arc_swap::ArcSwapOption;
use async_trait::async_trait;
use candid::Principal;
use ethnum::u256;
use tracing::{error, info};

use crate::{
    metrics::{MetricParamsPersist, WithMetricsPersist},
    snapshot::{Node, Subnet},
};

#[derive(Copy, Clone)]
pub struct PersistResults {
    pub ranges_old: u32,
    pub ranges_new: u32,
    pub nodes_old: u32,
    pub nodes_new: u32,
}

#[derive(Copy, Clone)]
pub enum PersistStatus {
    Completed(PersistResults),
    SkippedEmpty,
}

// Converts byte slice principal to a u256
fn principal_bytes_to_u256(p: &[u8]) -> u256 {
    if p.len() > 29 {
        panic!("Principal length should be <30 bytes");
    }

    // Since Principal length can be anything in 0..29 range - prepend it with zeros to 32
    let pad = 32 - p.len();
    let mut padded: [u8; 32] = [0; 32];
    padded[pad..32].copy_from_slice(p);

    u256::from_be_bytes(padded)
}

// Principals are 2^232 max so we can use the u256 type to efficiently store them
// Under the hood u256 is using two u128
// This is more efficient than lexographically sorted hexadecimal strings as done in JS router
// Currently the largest canister_id range is somewhere around 2^40 - so probably using one u128 would work for a long time
// But going u256 makes it future proof and according to spec
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteSubnet {
    pub id: String,
    pub range_start: u256,
    pub range_end: u256,
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Routes {
    pub node_count: u32,
    // subnets should be sorted by `range_start` field for the binary search to work
    pub subnets: Vec<Arc<RouteSubnet>>,
}

impl Routes {
    // Look up the subnet by canister_id
    pub fn lookup(&self, canister_id: Principal) -> Option<Arc<RouteSubnet>> {
        let canister_id_u256 = principal_bytes_to_u256(canister_id.as_slice());

        let idx = match self
            .subnets
            .binary_search_by_key(&canister_id_u256, |x| x.range_start)
        {
            // Ok should happen rarely when canister_id equals lower bound of some subnet
            Ok(i) => i,

            // In the Err case the returned value might be:
            // - index of next subnet to the one we look for (can be equal to vec len)
            // - 0 in case if canister_id is < than first subnet's range_start
            //
            // For case 1 we subtract the index to get subnet to check
            // Case 2 always leads to a lookup error, but this is handled in the next step
            Err(i) => {
                if i > 0 {
                    i - 1
                } else {
                    i
                }
            }
        };

        let subnet = self.subnets[idx].clone();
        if canister_id_u256 < subnet.range_start || canister_id_u256 > subnet.range_end {
            return None;
        }

        Some(subnet)
    }
}

#[async_trait]
pub trait Persist: Send + Sync {
    async fn persist(&self, subnets: Vec<Subnet>) -> PersistStatus;
}

pub struct Persister {
    published_routes: Arc<ArcSwapOption<Routes>>,
}

impl Persister {
    pub fn new(published_routes: Arc<ArcSwapOption<Routes>>) -> Self {
        Self { published_routes }
    }
}

#[async_trait]
impl Persist for Persister {
    // Construct a lookup table based on the provided subnet list
    async fn persist(&self, subnets: Vec<Subnet>) -> PersistStatus {
        if subnets.is_empty() {
            return PersistStatus::SkippedEmpty;
        }

        // Generate a list of subnets with a single canister range
        // Can contain several entries with the same subnet ID
        let mut rt_subnets = vec![];

        let mut node_count: u32 = 0;
        for subnet in subnets.into_iter() {
            node_count += subnet.nodes.len() as u32;

            for range in subnet.ranges.into_iter() {
                rt_subnets.push(Arc::new(RouteSubnet {
                    id: subnet.id.to_string(),
                    range_start: principal_bytes_to_u256(range.start.as_slice()),
                    range_end: principal_bytes_to_u256(range.end.as_slice()),
                    nodes: subnet.nodes.clone(),
                }))
            }
        }

        // Sort subnets by range_start for the binary search to work in lookup()
        rt_subnets.sort_by_key(|x| x.range_start);

        let rt = Arc::new(Routes {
            node_count,
            subnets: rt_subnets,
        });

        // Load old subnet to get previous numbers
        let rt_old = self.published_routes.load_full();
        let (ranges_old, nodes_old) =
            rt_old.map_or((0, 0), |x| (x.subnets.len() as u32, x.node_count));

        let results = PersistResults {
            ranges_old,
            ranges_new: rt.subnets.len() as u32,
            nodes_old,
            nodes_new: rt.node_count,
        };

        // Publish new routing table
        self.published_routes.store(Some(rt));

        PersistStatus::Completed(results)
    }
}

#[async_trait]
impl<T: Persist> Persist for WithMetricsPersist<T> {
    async fn persist(&self, subnets: Vec<Subnet>) -> PersistStatus {
        let out = self.0.persist(subnets).await;
        let MetricParamsPersist { nodes, ranges } = &self.1;

        match out {
            PersistStatus::SkippedEmpty => {
                error!("Lookup table is empty");
            }

            PersistStatus::Completed(s) => {
                nodes.set(s.nodes_new as i64);
                ranges.set(s.ranges_new as i64);

                info!(
                    action = "persist",
                    "Lookup table published: subnet ranges: {:?} -> {:?}, nodes: {:?} -> {:?}",
                    s.ranges_old,
                    s.ranges_new,
                    s.nodes_old,
                    s.nodes_new,
                );
            }
        }

        out
    }
}

#[cfg(test)]
pub mod test;

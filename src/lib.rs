pub mod adapters;
pub mod judge;
pub mod run_cycle;
pub mod ci144;
pub mod states;
pub mod reflex;
pub mod config;
pub mod hitl;
pub mod lifecycle;
pub mod task_dag;
pub mod gloves;
pub mod events;
pub mod trace;
pub mod health;
pub mod contract;
pub mod evidence;
pub mod criteria;
pub mod ledger;
pub mod pipeline;
pub mod rails;
pub mod security;

// Include gRPC auto-generated Helix-Mind API
pub mod helix_mind_api {
    tonic::include_proto!("helix_mind");
}

pub mod flowmodus_api {
    tonic::include_proto!("flowmodus");
}

pub mod tentacle_api {
    tonic::include_proto!("tentacle.v1");
}

pub mod adapters;
pub mod agent_loop;
pub mod states;
pub mod reflex;
pub mod config;

// Include gRPC auto-generated Helix-Mind API
pub mod helix_mind_api {
    tonic::include_proto!("helix_mind");
}

pub mod flowmodus_api {
    tonic::include_proto!("flowmodus");
}

pub mod tentacle_api {
    tonic::include_proto!("tentacle");
}

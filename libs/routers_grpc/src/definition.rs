/// The standard service definitions
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/_includes.rs"));

    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("routers_descriptor");
}

pub mod api {
    pub use super::proto::api::*;
}

pub mod model {
    pub use super::proto::model::v1::*;
}

pub mod optimise {
    pub use super::api::optimise::v1::optimisation_service_client::OptimisationServiceClient;
    pub use super::api::optimise::v1::optimisation_service_server::OptimisationService;
    pub use super::api::optimise::v1::optimisation_service_server::OptimisationServiceServer;

    pub use super::api::optimise::v1::*;
}

pub mod r#match {
    pub use super::api::r#match::v1::match_service_client::MatchServiceClient;
    pub use super::api::r#match::v1::match_service_server::MatchService;
    pub use super::api::r#match::v1::match_service_server::MatchServiceServer;

    pub use super::api::r#match::v1::*;
}

pub mod proximity {
    pub use super::api::proximity::v1::proximity_service_client::ProximityServiceClient;
    pub use super::api::proximity::v1::proximity_service_server::ProximityService;
    pub use super::api::proximity::v1::proximity_service_server::ProximityServiceServer;

    pub use super::api::proximity::v1::*;
}

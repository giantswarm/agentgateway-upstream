pub(crate) mod ateattr;
mod egress;
mod egress_actor_resolution;
mod ingress;

pub use egress::SubstrateEgress;
pub(crate) use egress::{EgressRequestProtocol, EgressTlsMode, authorize_tls};
pub(crate) use egress_actor_resolution::ActorIdentity;
pub use egress_actor_resolution::EgressActorResolution;
pub use ingress::SubstrateIngress;
pub(crate) use ingress::{
	STALE_ASSIGNMENT_HEADER, SubstrateRequestState, is_stale_assignment, stale_assignment_unavailable,
};

const CACHE_CAPACITY: usize = 10_000;
const TRACE_POLICY_KIND: &str = "substrate";

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct ActorRef {
	atespace: String,
	name: String,
}

/// Whether a failed call to a Substrate policy service (the actor identity check, the egress
/// policy, a credential provider) means the service could not be asked, rather than that it
/// refused. Besides the service's own Unavailable and DeadlineExceeded, a status built from a
/// local proxy error (a refused connect, a failed DNS lookup, no endpoint) never reached the
/// service: tonic reports it as Unknown, which must not read as a denial.
fn policy_service_unavailable(status: &tonic::Status) -> bool {
	matches!(
		status.code(),
		tonic::Code::Unavailable | tonic::Code::DeadlineExceeded
	) || std::error::Error::source(status)
		.is_some_and(|source| source.is::<crate::proxy::ProxyError>())
}

fn valid_resource_name(name: &str) -> bool {
	let bytes = name.as_bytes();
	(1..=63).contains(&bytes.len())
		&& bytes.first().is_some_and(u8::is_ascii_alphanumeric)
		&& bytes.last().is_some_and(u8::is_ascii_alphanumeric)
		&& bytes
			.iter()
			.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
}

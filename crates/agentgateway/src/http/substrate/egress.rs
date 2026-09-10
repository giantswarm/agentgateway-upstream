use serde::Deserialize;
use tonic::Code;

use super::{ActorRef, TRACE_POLICY_KIND, valid_resource_name};
use crate::http::Request;
use crate::proxy::httpproxy::PolicyClient;
use crate::proxy::{ProxyError, ProxyResponse};
use crate::telemetry::log;
use crate::telemetry::log::RequestLog;
use crate::telemetry::metrics::{OutboundCallKind, OutboundCallSubtype};
use crate::transport::stream::{Extension, TCPConnectionInfo, TLSConnectionInfo};
use crate::types::agent::SimpleBackendReferenceWithPolicies;
use crate::*;

const ACTOR_IDENTITY_OID: &str = "1.3.6.1.4.1.11129.2.12.2";

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ActorIdentity {
	atespace: String,
	actor_name: String,
	actor_uid: String,
	purpose: String,
}

/// Validates an actor's identity before accepting a CONNECT tunnel.
#[apply(schema!)]
pub struct SubstrateEgress {
	/// Backend that receives GetActor calls and policies used when connecting to it.
	#[serde(flatten)]
	pub target: SimpleBackendReferenceWithPolicies,
}

impl SubstrateEgress {
	fn identity(req: &Request) -> Result<ActorIdentity, ProxyError> {
		let certificate = req
			.extensions()
			.get::<TLSConnectionInfo>()
			.and_then(|tls| tls.src_identity.as_ref())
			.and_then(|identity| identity.certificate.as_deref())
			.ok_or_else(|| {
				ProxyError::SubstrateEgressDenied("missing authenticated actor certificate".to_owned())
			})?;
		let pem = pem::parse(certificate.as_bytes()).map_err(|error| {
			ProxyError::SubstrateEgressDenied(format!("invalid actor certificate: {error}"))
		})?;
		let (_, certificate) =
			x509_parser::parse_x509_certificate(pem.contents()).map_err(|error| {
				ProxyError::SubstrateEgressDenied(format!("invalid actor certificate: {error}"))
			})?;
		let mut extensions = certificate
			.extensions()
			.iter()
			.filter(|extension| extension.oid.to_id_string() == ACTOR_IDENTITY_OID);
		let extension = extensions.next().ok_or_else(|| {
			ProxyError::SubstrateEgressDenied("actor certificate has no ActorIdentity".to_owned())
		})?;
		if extensions.next().is_some() {
			return Err(ProxyError::SubstrateEgressDenied(
				"actor certificate has multiple ActorIdentity extensions".to_owned(),
			));
		}
		let identity: ActorIdentity = serde_json::from_slice(extension.value).map_err(|error| {
			ProxyError::SubstrateEgressDenied(format!("invalid ActorIdentity: {error}"))
		})?;
		if !valid_resource_name(&identity.atespace)
			|| !valid_resource_name(&identity.actor_name)
			|| identity.actor_uid.is_empty()
			|| identity.purpose != "atunnel"
		{
			return Err(ProxyError::SubstrateEgressDenied(
				"invalid ActorIdentity".to_owned(),
			));
		}
		Ok(identity)
	}

	pub(crate) async fn authorize_connect(
		&self,
		inputs: &Arc<ProxyInputs>,
		connection: &Extension,
		req: &mut Request,
	) -> Result<(), ProxyResponse> {
		let tcp = connection
			.copy::<TCPConnectionInfo>(req.extensions_mut())
			.expect("tcp connection must be set")
			.clone();
		connection.copy::<TLSConnectionInfo>(req.extensions_mut());
		let mut log = RequestLog::new(
			log::CelLogging::new(inputs.cfg.logging.clone(), inputs.cfg.metrics.clone()),
			inputs.metrics.clone(),
			inputs.model_catalog.clone(),
			agent_core::Timestamp::now(),
			tcp,
		);
		self
			.authorize(
				&PolicyClient::new(inputs.clone()).with_parent(req),
				&mut log,
				req,
			)
			.await
	}

	async fn authorize(
		&self,
		client: &PolicyClient,
		log: &mut RequestLog,
		req: &mut Request,
	) -> Result<(), ProxyResponse> {
		let identity = Self::identity(req)?;
		let actor = ActorRef {
			atespace: identity.atespace,
			name: identity.actor_name,
		};
		log.ate_actor_id = Some(actor.name.clone());
		log.ate_atespace = Some(actor.atespace.clone());
		let channel = self
			.target
			.grpc_channel(client.with_outbound(OutboundCallKind::Policy, OutboundCallSubtype::Substrate));
		let mut control = protos::ateapi::control_client::ControlClient::new(channel);
		let result = crate::proxy::dtrace::scope_future(
			Some(TRACE_POLICY_KIND),
			control.get_actor(protos::ateapi::GetActorRequest {
				actor: Some(protos::ateapi::ObjectRef {
					atespace: actor.atespace.clone(),
					name: actor.name.clone(),
				}),
			}),
		)
		.await;
		let current = match result {
			Ok(response) => response.into_inner(),
			Err(status) if matches!(status.code(), Code::Unavailable | Code::DeadlineExceeded) => {
				return Err(
					ProxyError::SubstrateEgressUnavailable(format!(
						"actor identity check unavailable: {status}"
					))
					.into(),
				);
			},
			Err(status) => {
				return Err(
					ProxyError::SubstrateEgressDenied(format!("actor identity check denied: {status}"))
						.into(),
				);
			},
		};
		if current
			.metadata
			.as_ref()
			.map(|metadata| metadata.uid.as_str())
			!= Some(identity.actor_uid.as_str())
		{
			return Err(ProxyError::SubstrateEgressDenied("actor UID mismatch".to_owned()).into());
		}
		// The actor must be placed on a worker. Running is the steady state;
		// Resuming is the workload booting or restoring on the worker that
		// minted this certificate for exactly that placement (ate-api mints
		// only for a worker's current assignment), and what a workload
		// fetches to become ready -- models, skills, packages -- goes out
		// before it serves readyz. Every other state means the actor has left
		// its worker or is leaving it: a certificate still within its lifetime
		// must not open a tunnel on its behalf.
		let state = current.status.as_ref().map(|status| status.state);
		if !placed_on_worker(state) {
			return Err(
				ProxyError::SubstrateEgressDenied(format!(
					"actor is not placed on a worker (state {})",
					state
						.and_then(|state| protos::ateapi::ActorState::try_from(state).ok())
						.map(|state| state.as_str_name().to_owned())
						.unwrap_or_else(|| "unknown".to_owned())
				))
				.into(),
			);
		}
		Ok(())
	}
}

/// Whether an actor in `state` is placed on a worker and may open a tunnel:
/// `Running`, or `Resuming` onto the worker that minted its certificate.
fn placed_on_worker(state: Option<i32>) -> bool {
	matches!(
		state.and_then(|state| protos::ateapi::ActorState::try_from(state).ok()),
		Some(protos::ateapi::ActorState::Running | protos::ateapi::ActorState::Resuming)
	)
}

#[cfg(test)]
mod tests {
	use rcgen::{CertificateParams, CustomExtension, KeyPair};

	use super::*;
	use crate::http::Body;
	use crate::transport::tls::TlsInfo;

	fn request_with_identity(identity: &str) -> Request {
		let mut params = CertificateParams::default();
		params
			.custom_extensions
			.push(CustomExtension::from_oid_content(
				&[1, 3, 6, 1, 4, 1, 11129, 2, 12, 2],
				identity.as_bytes().to_vec(),
			));
		let certificate = params
			.self_signed(&KeyPair::generate().unwrap())
			.unwrap()
			.pem();
		let mut req = Request::new(Body::empty());
		req.extensions_mut().insert(TLSConnectionInfo {
			src_identity: Some(TlsInfo {
				certificate: Some(certificate.into()),
				..Default::default()
			}),
			..Default::default()
		});
		req
	}

	#[test]
	fn only_placed_actors_may_tunnel() {
		use protos::ateapi::ActorState;
		assert!(placed_on_worker(Some(ActorState::Running as i32)));
		assert!(placed_on_worker(Some(ActorState::Resuming as i32)));
		for state in [
			ActorState::Unspecified,
			ActorState::Suspending,
			ActorState::Suspended,
			ActorState::Pausing,
			ActorState::Paused,
			ActorState::Crashed,
			ActorState::Deleting,
		] {
			assert!(!placed_on_worker(Some(state as i32)), "{state:?}");
		}
		assert!(!placed_on_worker(None));
		assert!(!placed_on_worker(Some(-1)));
	}

	#[test]
	fn actor_identity_is_parsed_from_the_certificate() {
		let identity = SubstrateEgress::identity(&request_with_identity(
			r#"{"Atespace":"demo","ActorName":"my-actor","ActorUid":"uid-1","Purpose":"atunnel"}"#,
		))
		.unwrap();
		assert_eq!(identity.atespace, "demo");
		assert_eq!(identity.actor_name, "my-actor");
		assert_eq!(identity.actor_uid, "uid-1");
	}

	#[test]
	fn actor_identity_requires_every_field_and_atunnel_purpose() {
		for identity in [
			r#"{"Atespace":"","ActorName":"my-actor","ActorUid":"uid-1","Purpose":"atunnel"}"#,
			r#"{"Atespace":"demo","ActorName":"","ActorUid":"uid-1","Purpose":"atunnel"}"#,
			r#"{"Atespace":"demo","ActorName":"my-actor","ActorUid":"","Purpose":"atunnel"}"#,
			r#"{"Atespace":"demo","ActorName":"my-actor","ActorUid":"uid-1","Purpose":"other"}"#,
		] {
			assert!(SubstrateEgress::identity(&request_with_identity(identity)).is_err());
		}
	}
}

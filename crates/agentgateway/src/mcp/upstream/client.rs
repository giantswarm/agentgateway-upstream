use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use crate::client::ResolvedDestination;
use crate::proxy::ProxyError;
use crate::proxy::httpproxy::PolicyClient;
use crate::store::BackendPolicies;
use crate::telemetry::metrics::{OutboundCallKind, OutboundCallSubtype};
use crate::types::agent::SimpleBackend;

/// HTTP client for MCP upstream backends with optional stateful session affinity.
/// Used by all HTTP-based MCP upstream transports (SSE, StreamableHTTP, OpenAPI).
///
/// When `stateful` is true, this client captures the resolved backend address
/// from the first response and pins all subsequent requests to that same endpoint.
/// This ensures session affinity for stateful MCP backends with multiple replicas.
/// The pin lasts as long as its endpoint: once the endpoint leaves the service, or a dial
/// to it fails, the pin is dropped and the next request resolves the backend afresh.
#[derive(Debug, Clone)]
pub(crate) struct McpHttpClient {
	client: PolicyClient,
	backend: Arc<SimpleBackend>,
	base_policies: BackendPolicies,
	pinned_dest: Arc<Mutex<Option<ResolvedDestination>>>,
	stateful: bool,
	target_name: String,
}

impl McpHttpClient {
	pub fn new(
		client: PolicyClient,
		backend: SimpleBackend,
		policies: BackendPolicies,
		stateful: bool,
		target_name: String,
	) -> Self {
		Self {
			client,
			backend: Arc::new(backend),
			base_policies: policies,
			pinned_dest: Arc::new(Mutex::new(None)),
			stateful,
			target_name,
		}
	}

	pub async fn call(
		&self,
		req: crate::http::Request,
	) -> Result<http::Response<crate::http::Body>, ProxyError> {
		let client = self.client.with_parent(&req);
		let mut policies = self.base_policies.clone();

		let pinned = if self.stateful && policies.override_dest.is_none() {
			self.live_pin()
		} else {
			None
		};
		if let Some(pinned) = pinned {
			tracing::trace!(
				target = %self.target_name,
				backend = %self.backend,
				endpoint = %pinned,
				"using pinned backend endpoint"
			);
			policies.override_dest = Some(pinned);
		}

		let resp = Box::pin(
			client
				.with_outbound(OutboundCallKind::Primary, OutboundCallSubtype::Mcp)
				.with_dtrace_scope(format!("mcp {}", self.target_name))
				// The Upstream layer owns the semantic MCP span so it can include the MCP method,
				// target, and tool name. Tracing this HTTP transport would create a duplicate span.
				.call_with_explicit_policies_untraced(req, &self.backend, policies),
		)
		.await;
		let resp = match resp {
			Err(err) if pinned.is_some() && is_dial_failure(&err) => {
				// The pinned endpoint is unreachable: the next request resolves the backend afresh.
				self.unpin(pinned, &err);
				return Err(err);
			},
			resp => resp?,
		};

		// Capture resolved destination on first request if stateful
		if self.stateful
			&& self.pinned_dest.lock().unwrap().is_none()
		// Only pin to services. Pinning to a specific IP for DNS resolution is not appropriate.
		// With Service, we know there are replicas of the pod and we can pin to that.
		&& let SimpleBackend::Service(_, _) = &*self.backend
			&& let Some(resolved) = resp.extensions().get::<ResolvedDestination>()
		{
			self.pin_backend(*resolved);
		}

		Ok(resp)
	}

	pub fn pin_backend(&self, resolved: ResolvedDestination) {
		tracing::debug!(
			target = %self.target_name,
			backend = %self.backend,
			endpoint = %resolved.0,
			"pinned stateful MCP session to backend endpoint"
		);
		*self.pinned_dest.lock().unwrap() = Some(resolved);
	}

	/// Returns the pinned endpoint while the backend still has it; drops a pin whose endpoint has
	/// left the service, so the request is balanced across the live endpoints instead.
	fn live_pin(&self) -> Option<SocketAddr> {
		let pinned = self.pinned_backend()?;
		if let SimpleBackend::Service(svc, _) = &*self.backend {
			let discovery = self.client.inputs.stores.read_discovery();
			if !svc.endpoints.has_override(&discovery.workloads, pinned) {
				drop(discovery);
				self.unpin(Some(pinned), "endpoint is no longer part of the service");
				return None;
			}
		}
		Some(pinned)
	}

	/// Drops the pin if it is still `pinned`; a pin a concurrent request set since is kept.
	fn unpin(&self, pinned: Option<SocketAddr>, reason: impl std::fmt::Display) {
		let mut current = self.pinned_dest.lock().unwrap();
		if current.map(|c| c.0) != pinned {
			return;
		}
		tracing::info!(
			target = %self.target_name,
			backend = %self.backend,
			endpoint = ?pinned,
			%reason,
			"unpinned stateful MCP session from backend endpoint"
		);
		*current = None;
	}

	pub fn pinned_backend(&self) -> Option<SocketAddr> {
		Some((*self.pinned_dest.lock().unwrap())?.0)
	}

	pub fn target_name(&self) -> &str {
		&self.target_name
	}

	pub fn backend(&self) -> &SimpleBackend {
		&self.backend
	}
}

/// Returns true if the request failed before it reached an upstream endpoint: no endpoint matched,
/// or the connection to it could not be established.
fn is_dial_failure(err: &ProxyError) -> bool {
	match err {
		ProxyError::NoHealthyEndpoints | ProxyError::UpstreamConnectTimeout => true,
		ProxyError::UpstreamCallFailed(err) => err.is_connect(),
		_ => false,
	}
}

#[cfg(test)]
mod tests {
	use std::collections::HashMap;
	use std::sync::Arc;

	use agent_core::strng;
	use wiremock::matchers::method;
	use wiremock::{Mock, MockServer, ResponseTemplate};

	use super::*;
	use crate::ProxyInputs;
	use crate::store::{DiscoveryPreviousState as PreviousState, LocalWorkload};
	use crate::test_helpers::proxymock::setup_proxy_test;
	use crate::types::discovery::{NamespacedHostname, NetworkAddress, Service, Workload};

	const SVC_KEY: &str = "default/mcp.default.svc.cluster.local";

	/// A pod of the MCP service: a server that answers every request with its name.
	async fn pod(name: &str) -> MockServer {
		// Not from wiremock's pool: a dropped pod must stop listening.
		let server = MockServer::builder().start().await;
		Mock::given(method("POST"))
			.respond_with(ResponseTemplate::new(200).set_body_string(name))
			.mount(&server)
			.await;
		server
	}

	/// Makes `pods` the service's endpoints, replacing the previous set.
	fn set_endpoints(
		pi: &ProxyInputs,
		pods: &[(&str, SocketAddr)],
		prev: PreviousState,
	) -> PreviousState {
		let svc = Service {
			name: strng::literal!("mcp"),
			namespace: strng::literal!("default"),
			hostname: strng::literal!("mcp.default.svc.cluster.local"),
			vips: vec![NetworkAddress {
				network: strng::EMPTY,
				address: "10.0.0.1".parse().unwrap(),
			}],
			ports: HashMap::from([(80, 0)]),
			..Default::default()
		};
		let workloads = pods
			.iter()
			.map(|(name, addr)| LocalWorkload {
				workload: Workload {
					uid: strng::new(name),
					name: strng::new(name),
					namespace: strng::literal!("default"),
					workload_ips: vec![addr.ip()],
					..Default::default()
				},
				services: HashMap::from([(SVC_KEY.to_string(), HashMap::from([(80, addr.port())]))]),
			})
			.collect();
		pi.stores
			.discovery
			.sync_local(vec![svc], workloads, prev)
			.unwrap()
	}

	fn stateful_client(pi: Arc<ProxyInputs>) -> McpHttpClient {
		let svc = pi
			.stores
			.read_discovery()
			.services
			.get_by_namespaced_host(&NamespacedHostname {
				namespace: strng::literal!("default"),
				hostname: strng::literal!("mcp.default.svc.cluster.local"),
			})
			.expect("service is in the store");
		McpHttpClient::new(
			PolicyClient::new(pi),
			SimpleBackend::Service(svc, 80),
			BackendPolicies::default(),
			true,
			"mcp".to_string(),
		)
	}

	/// Sends one request of the session and returns the name of the pod that answered it.
	async fn send(client: &McpHttpClient) -> Result<String, ProxyError> {
		let req = ::http::Request::builder()
			.method(::http::Method::POST)
			.uri("http://mcp.default.svc.cluster.local/mcp")
			.body(crate::http::Body::from("{}"))
			.unwrap();
		let resp = client.call(req).await?;
		let body = crate::http::tests_common::read_body_for_test(resp.into_body()).await;
		Ok(String::from_utf8(body.to_vec()).unwrap())
	}

	#[tokio::test]
	async fn session_follows_its_pod_replacement() {
		let t = setup_proxy_test("{}").unwrap();
		let a = pod("a").await;
		let prev = set_endpoints(&t.pi, &[("a", *a.address())], Default::default());
		let client = stateful_client(t.pi.clone());

		assert_eq!(send(&client).await.unwrap(), "a");
		assert_eq!(client.pinned_backend(), Some(*a.address()));

		// Pod a is evicted and replaced by pod b.
		let b = pod("b").await;
		drop(a);
		set_endpoints(&t.pi, &[("b", *b.address())], prev);

		assert_eq!(send(&client).await.unwrap(), "b");
		assert_eq!(client.pinned_backend(), Some(*b.address()));
		assert_eq!(send(&client).await.unwrap(), "b");
	}

	#[tokio::test]
	async fn session_stays_pinned_to_a_healthy_pod() {
		let t = setup_proxy_test("{}").unwrap();
		let a = pod("a").await;
		let b = pod("b").await;
		set_endpoints(
			&t.pi,
			&[("a", *a.address()), ("b", *b.address())],
			Default::default(),
		);
		let client = stateful_client(t.pi.clone());

		let first = send(&client).await.unwrap();
		let pinned = client.pinned_backend();
		for _ in 0..20 {
			assert_eq!(send(&client).await.unwrap(), first);
			assert_eq!(client.pinned_backend(), pinned);
		}
	}

	#[tokio::test]
	async fn failed_dial_to_the_pinned_pod_drops_the_pin() {
		let t = setup_proxy_test("{}").unwrap();
		// Pod a is gone while the service still lists it: the dial is refused.
		let gone = std::net::TcpListener::bind("127.0.0.1:0")
			.unwrap()
			.local_addr()
			.unwrap();
		set_endpoints(&t.pi, &[("a", gone)], Default::default());
		let client = stateful_client(t.pi.clone());
		client.pin_backend(ResolvedDestination(gone));

		let err = send(&client).await.unwrap_err();
		assert!(is_dial_failure(&err), "unexpected error: {err:?}");
		assert_eq!(client.pinned_backend(), None);
	}

	#[test]
	fn only_failures_before_the_upstream_count_as_dial_failures() {
		assert!(is_dial_failure(&ProxyError::NoHealthyEndpoints));
		assert!(is_dial_failure(&ProxyError::UpstreamConnectTimeout));
		assert!(!is_dial_failure(&ProxyError::UpstreamCallTimeout));
		assert!(!is_dial_failure(&ProxyError::RequestTimeout));
	}
}

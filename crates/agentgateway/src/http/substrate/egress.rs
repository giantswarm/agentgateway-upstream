use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use ::http::{HeaderName, HeaderValue};
use quick_cache::sync::Cache;
use tonic::Code;

use super::{ActorIdentity, EgressActorResolution, TRACE_POLICY_KIND};
use crate::http::{PolicyResponse, Request};
use crate::proxy::httpproxy::{DynamicBackendOverride, PolicyClient};
use crate::proxy::{ProxyError, ProxyResponse};
use crate::store::RequestPolicyTrait;
use crate::telemetry::log::RequestLog;
use crate::telemetry::metrics::{OutboundCallKind, OutboundCallSubtype};
use crate::transport::stream::{Extension, TCPConnectionInfo};
use crate::types::agent::{SimpleBackendReferenceWithPolicies, Target};
use crate::{cel, *};

const DEFAULT_CREDENTIAL_CACHE_CAPACITY: usize = 8192;
const DEFAULT_CREDENTIAL_CACHE_TTL: Duration = Duration::from_secs(300);
const CREDENTIAL_FETCH_TIMEOUT: Duration = Duration::from_secs(4);

/// The inner HTTP transport, independent of the actor's CONNECT transport or URI scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EgressRequestProtocol {
	Http,
	Https,
}

/// Retrieves and enforces the current Substrate egress policy for each request.
#[apply(schema!)]
pub struct SubstrateEgress {
	/// Backend that receives GetActorEgressPolicy calls and policies used when connecting to it.
	#[serde(flatten)]
	pub target: SimpleBackendReferenceWithPolicies,
	/// Credential providers available to secret-backed egress effects, keyed by
	/// the authority in an `ate-secret://` URI.
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub credential_providers: Vec<CredentialProvider>,
	#[serde(skip, default = "default_credential_cache")]
	#[cfg_attr(feature = "schema", schemars(skip))]
	credential_cache: CredentialCache,
}

/// An inline credential-provider backend selected by credential URI authority.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct CredentialProvider {
	/// Exact credential URI authority handled by this provider, such as `kubernetes.io`.
	#[serde(rename = "uriAuthority")]
	pub uri_authority: String,
	/// Backend that resolves credentials and policies used when connecting to it.
	pub target: SimpleBackendReferenceWithPolicies,
	/// How long to reuse a fetched credential. Defaults to 5 minutes; `0s` disables caching.
	#[serde(
		default,
		with = "crate::serdes::serde_dur_option",
		skip_serializing_if = "Option::is_none"
	)]
	#[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
	pub cache_ttl: Option<Duration>,
}

#[derive(Debug, Clone)]
struct CredentialCache {
	entries: Arc<Cache<CredentialCacheKey, CachedCredential>>,
}

#[derive(Clone, Hash, PartialEq, Eq)]
struct CredentialCacheKey {
	actor_identity: String,
	actor_uid: Option<String>,
	uri: String,
}

#[derive(Clone)]
struct CachedCredential {
	secret: Vec<u8>,
	fetched_at: Instant,
}

impl CredentialCache {
	fn new(capacity: usize) -> Self {
		Self {
			entries: Arc::new(Cache::new(capacity)),
		}
	}

	fn get(&self, key: &CredentialCacheKey, now: Instant, ttl: Duration) -> Option<Vec<u8>> {
		if let Some(entry) = self.entries.get(key) {
			if now.duration_since(entry.fetched_at) <= ttl {
				return Some(entry.secret);
			}
			self
				.entries
				.remove_if(key, |entry| now.duration_since(entry.fetched_at) > ttl);
		}
		None
	}

	fn insert(&self, key: CredentialCacheKey, secret: Vec<u8>, now: Instant) {
		self.entries.insert(
			key,
			CachedCredential {
				secret,
				fetched_at: now,
			},
		);
	}
}

fn default_credential_cache() -> CredentialCache {
	CredentialCache::new(DEFAULT_CREDENTIAL_CACHE_CAPACITY)
}

impl RequestPolicyTrait for SubstrateEgress {
	async fn apply(
		&self,
		client: &PolicyClient,
		log: &mut RequestLog,
		req: &mut Request,
	) -> Result<PolicyResponse, ProxyResponse> {
		let identity = req
			.extensions()
			.get::<ActorIdentity>()
			.cloned()
			.ok_or_else(|| {
				ProxyError::SubstrateEgressDenied("missing CONNECT-authorized actor identity".to_owned())
			})?;
		log.ate_actor_name = Some(identity.actor_name.clone());
		log.ate_actor_uid = identity.actor_uid.clone();
		log.ate_atespace = Some(identity.atespace.clone());
		let policy = fetch_policy(&self.target, client, &identity).await?;
		let matched_rule = matching_rule(&policy, req)?;
		// Dynamic forwarding uses the authorized name and the original destination
		// port. A port supplied in the HTTP authority must not change the dial target.
		let destination = req
			.extensions()
			.get::<cel::DestinationContext>()
			.expect("validated destination");
		let hostname = destination.hostname.as_deref().expect("validated hostname");
		let hostname = hostname
			.strip_prefix('[')
			.and_then(|host| host.strip_suffix(']'))
			.unwrap_or(hostname);
		let target = Target::from((hostname, destination.port));
		req.extensions_mut().insert(DynamicBackendOverride(target));
		self
			.apply_effects(
				client,
				&identity,
				matched_rule
					.http
					.as_ref()
					.and_then(|rule| rule.effects.as_ref())
					.or_else(|| {
						matched_rule
							.https
							.as_ref()
							.and_then(|rule| rule.effects.as_ref())
					}),
				req,
			)
			.await?;
		Ok(PolicyResponse::default())
	}
}

pub(crate) fn reject_upgrade(req: &Request) -> Result<(), ProxyError> {
	if req.method() != ::http::Method::CONNECT && !req.headers().contains_key(::http::header::UPGRADE)
	{
		return Ok(());
	}
	let websocket = req
		.headers()
		.get_all(::http::header::UPGRADE)
		.iter()
		.filter_map(|value| value.to_str().ok())
		.flat_map(|value| value.split(','))
		.any(|token| token.trim().eq_ignore_ascii_case("websocket"));
	Err(ProxyError::SubstrateEgressUpgradeDenied(if websocket {
		"WebSocket egress is not supported"
	} else if req.method() == ::http::Method::CONNECT {
		"CONNECT egress is not supported"
	} else {
		"HTTP upgrades, including CONNECT, are denied for actor egress"
	}))
}

impl SubstrateEgress {
	async fn apply_effects(
		&self,
		client: &PolicyClient,
		identity: &ActorIdentity,
		effects: Option<&protos::ateapi::HttpRuleEffects>,
		req: &mut Request,
	) -> Result<(), ProxyResponse> {
		let Some(effects) = effects else {
			return Ok(());
		};
		for injection in replace_headers_for(&effects.replace_headers, req.uri().path()) {
			if !req.headers().contains_key(&injection.header) {
				continue;
			}
			let secret = match (injection.credential_uri.as_str(), &injection.actor_jwt) {
				(uri, None) if !uri.is_empty() => {
					let provider = self.provider_for_uri(uri)?;
					self.credential(client, identity, provider, uri).await?
				},
				("", Some(source)) => self.actor_jwt(client, identity, source).await?,
				_ => {
					return Err(
						ProxyError::SubstrateEgressDenied(
							"credential effect must specify exactly one credential source".to_owned(),
						)
						.into(),
					);
				},
			};
			let (name, value) = credential_header(injection, secret)?;
			req.headers_mut().insert(name, value);
		}
		Ok(())
	}

	async fn actor_jwt(
		&self,
		client: &PolicyClient,
		identity: &ActorIdentity,
		source: &protos::ateapi::ActorJwtSource,
	) -> Result<Vec<u8>, ProxyResponse> {
		let mut audiences = source.audiences.clone();
		audiences.sort_unstable();
		let key = CredentialCacheKey {
			actor_identity: actor_spiffe_uri(&identity.atespace, &identity.actor_name),
			actor_uid: identity.actor_uid.clone(),
			uri: format!("actor-jwt:{}:{audiences:?}", source.expiration_seconds),
		};
		let ttl = Duration::from_secs(
			u64::try_from(source.expiration_seconds)
				.unwrap_or_default()
				.saturating_mul(2)
				/ 3,
		);
		if let Some(jwt) = self.credential_cache.get(&key, Instant::now(), ttl) {
			return Ok(jwt);
		}

		let policy_client =
			client.with_outbound(OutboundCallKind::Policy, OutboundCallSubtype::Substrate);
		let channel = self.target.grpc_channel(policy_client.clone());
		let mut request = tonic::Request::new(protos::ateapi::MintActorJwtRequest {
			actor: Some(protos::ateapi::ObjectRef {
				atespace: identity.atespace.clone(),
				name: identity.actor_name.clone(),
			}),
			audiences: source.audiences.clone(),
			expiration_seconds: source.expiration_seconds,
		});
		let mut span = policy_client.start_grpc_span(
			&mut request,
			self.target.target.as_ref(),
			"/substrate.ateapi.v1.Control/MintActorJwt",
		);
		let mut control = protos::ateapi::control_client::ControlClient::new(channel);
		let requested_at = Instant::now();
		let response = control.mint_actor_jwt(request).await;
		if let Some(span) = span.as_deref_mut() {
			span.record_grpc_result(&response);
		}
		drop(span);
		let response = response.map_err(actor_jwt_error)?.into_inner();
		let jwt = credential_secret(response.actor_jwt.into_bytes())?;
		self.credential_cache.insert(key, jwt.clone(), requested_at);
		Ok(jwt)
	}

	async fn credential(
		&self,
		client: &PolicyClient,
		identity: &ActorIdentity,
		provider: &CredentialProvider,
		uri: &str,
	) -> Result<Vec<u8>, ProxyResponse> {
		let actor_identity = actor_spiffe_uri(&identity.atespace, &identity.actor_name);
		let key = CredentialCacheKey {
			actor_identity: actor_identity.clone(),
			actor_uid: identity.actor_uid.clone(),
			uri: uri.to_owned(),
		};
		let ttl = provider.cache_ttl.unwrap_or(DEFAULT_CREDENTIAL_CACHE_TTL);
		if let Some(secret) = self.credential_cache.get(&key, Instant::now(), ttl) {
			return Ok(secret);
		}

		let policy_client =
			client.with_outbound(OutboundCallKind::Policy, OutboundCallSubtype::Substrate);
		let channel = provider.target.grpc_channel(policy_client.clone());
		let mut request = tonic::Request::new(protos::credprovider::FetchSecretRequest {
			uri: uri.to_owned(),
			actor_spiffe_id: actor_identity,
		});
		let mut span = policy_client.start_grpc_span(
			&mut request,
			provider.target.target.as_ref(),
			"/substrate.credprovider.v1.CredentialProvider/FetchSecret",
		);
		let mut provider =
			protos::credprovider::credential_provider_client::CredentialProviderClient::new(channel);
		let response = tokio::time::timeout(CREDENTIAL_FETCH_TIMEOUT, provider.fetch_secret(request))
			.await
			.unwrap_or_else(|_| Err(tonic::Status::deadline_exceeded("FetchSecret timed out")));
		if let Some(span) = span.as_deref_mut() {
			span.record_grpc_result(&response);
		}
		drop(span);
		let response = response
			.map_err(|status| credential_provider_error(uri, status))?
			.into_inner();
		let secret = credential_secret(response.opaque_bytes)?;
		if !ttl.is_zero() {
			self
				.credential_cache
				.insert(key, secret.clone(), Instant::now());
		}
		Ok(secret)
	}

	fn provider_for_uri(&self, uri: &str) -> Result<&CredentialProvider, ProxyResponse> {
		let name = provider_name(uri)
			.ok_or_else(|| ProxyError::SubstrateEgressDenied(format!("invalid credential URI: {uri}")))?;
		self
			.credential_providers
			.iter()
			.find(|provider| provider.uri_authority == name)
			.ok_or_else(|| {
				ProxyError::SubstrateEgressDenied(format!("no credential provider configured for {name}"))
			})
			.map_err(Into::into)
	}
}

/// Selects the `replace_headers` entries that apply to a request for `path`, at most one per header
/// name (compared case-insensitively), in the order the names first appear: the entry with the
/// longest `path_prefix` the path lies under, the entry without a prefix otherwise, none if neither.
/// A path that is not canonical (see [`canonical_path`]) matches no prefix. Same rules as
/// Substrate's `egresspolicy.ReplaceHeadersFor`.
fn replace_headers_for<'a>(
	replace_headers: &'a [protos::ateapi::CredentialHeader],
	path: &str,
) -> Vec<&'a protos::ateapi::CredentialHeader> {
	let canonical = canonical_path(path);
	let mut chosen: Vec<(String, Option<&protos::ateapi::CredentialHeader>)> = Vec::new();
	for injection in replace_headers {
		let name = injection.header.to_ascii_lowercase();
		let index = match chosen.iter().position(|(seen, _)| *seen == name) {
			Some(index) => index,
			None => {
				chosen.push((name, None));
				chosen.len() - 1
			},
		};
		let prefix = injection.path_prefix.as_str();
		if !prefix.is_empty() && !(canonical && path_prefix_matches(prefix, path)) {
			continue;
		}
		let entry = &mut chosen[index].1;
		if entry.is_none_or(|previous| prefix.len() > previous.path_prefix.len()) {
			*entry = Some(injection);
		}
	}
	chosen.into_iter().filter_map(|(_, entry)| entry).collect()
}

/// Whether `path` lies under `prefix`: it equals the prefix or continues it at a segment boundary.
/// "/api" matches "/api" and "/api/v4" but not "/apis"; "/api/" matches "/api/v4" but not "/api".
fn path_prefix_matches(prefix: &str, path: &str) -> bool {
	path
		.strip_prefix(prefix)
		.is_some_and(|rest| rest.is_empty() || prefix.ends_with('/') || rest.starts_with('/'))
}

/// Whether an absolute path reads the same to every origin. Prefixes compare byte for byte, so a
/// path with a dot or empty segment, a backslash, or a percent-encoded '.' or '\' could resolve
/// outside a prefix it starts with; such a path is not canonical. A percent-encoded '/' names one
/// segment to most origins (GitLab's `projects/<group>%2F<project>`) and a separator to some, so a
/// path carrying one is canonical only if it has no dot or empty segment read either way.
fn canonical_path(path: &str) -> bool {
	let Some(rest) = path.strip_prefix('/') else {
		return false;
	};
	if path
		.bytes()
		.any(|byte| byte <= b' ' || byte >= 0x7f || matches!(byte, b'\\' | b'?' | b'#'))
	{
		return false;
	}
	let lower = rest.to_ascii_lowercase();
	if ["%2e", "%5c"].iter().any(|encoded| lower.contains(encoded)) {
		return false;
	}
	let decoded = lower.replace("%2f", "/");
	let mut segments = decoded.split('/').peekable();
	while let Some(segment) = segments.next() {
		if segment.is_empty() && segments.peek().is_some() {
			return false;
		}
		// A ';' starts path parameters, which some origins strip before resolving dot segments.
		let name = segment.split(';').next().unwrap_or_default();
		if name == "." || name == ".." {
			return false;
		}
	}
	true
}

fn provider_name(uri: &str) -> Option<&str> {
	let authority = uri.strip_prefix("ate-secret://")?.split('/').next()?;
	(!authority.is_empty() && !authority.contains(['?', '#', '@', ':'])).then_some(authority)
}

fn actor_spiffe_uri(atespace: &str, actor_name: &str) -> String {
	// Translate the authenticated worker identity to the actor identity providers authorize.
	// Matches Substrate's resources.ActorSPIFFEID.
	format!("spiffe://substrate-actor.local/actor/{atespace}/{actor_name}")
}

fn credential_header(
	injection: &protos::ateapi::CredentialHeader,
	secret: Vec<u8>,
) -> Result<(HeaderName, HeaderValue), ProxyResponse> {
	let name = HeaderName::from_str(&injection.header).map_err(|error| {
		ProxyError::SubstrateEgressDenied(format!("invalid credential header: {error}"))
	})?;
	if protected_credential_header(&name) {
		return Err(
			ProxyError::SubstrateEgressDenied(format!(
				"credential effects cannot modify protected header {name}"
			))
			.into(),
		);
	}
	let secret = credential_secret(secret)?;
	let mut value = injection.prefix.as_bytes().to_vec();
	value.extend(&secret);
	let mut value = HeaderValue::from_bytes(&value).map_err(|error| {
		ProxyError::SubstrateEgressFailed(format!("credential header value is invalid: {error}"))
	})?;
	value.set_sensitive(true);
	Ok((name, value))
}

fn protected_credential_header(name: &HeaderName) -> bool {
	matches!(
		name.as_str(),
		"host"
			| "content-length"
			| "connection"
			| "keep-alive"
			| "proxy-authenticate"
			| "proxy-authorization"
			| "te"
			| "trailer"
			| "transfer-encoding"
			| "upgrade"
	)
}

/// 403 when the provider refuses the credential or its URI, 503 when a retry
/// may help, 502 for anything else (usually a bug in the provider).
fn credential_provider_error(uri: &str, status: tonic::Status) -> ProxyError {
	let provider = provider_name(uri).unwrap_or("unknown");
	match status.code() {
		Code::NotFound | Code::PermissionDenied | Code::FailedPrecondition | Code::InvalidArgument => {
			ProxyError::SubstrateEgressDenied(format!("credential provider {provider} denied: {status}"))
		},
		Code::Unavailable | Code::DeadlineExceeded | Code::ResourceExhausted => {
			ProxyError::SubstrateEgressUnavailable(format!(
				"credential provider {provider} unavailable: {status}"
			))
		},
		_ => {
			ProxyError::SubstrateEgressFailed(format!("credential provider {provider} failed: {status}"))
		},
	}
}

fn actor_jwt_error(status: tonic::Status) -> ProxyError {
	match status.code() {
		Code::Unavailable | Code::DeadlineExceeded => {
			ProxyError::SubstrateEgressUnavailable(format!("actor JWT issuer unavailable: {status}"))
		},
		_ => ProxyError::SubstrateEgressDenied(format!("actor JWT denied: {status}")),
	}
}

fn credential_secret(secret: Vec<u8>) -> Result<Vec<u8>, ProxyResponse> {
	let secret = secret.strip_suffix(b"\n").unwrap_or(&secret);
	let secret = secret.strip_suffix(b"\r").unwrap_or(secret);
	if secret.is_empty() || secret.iter().any(|byte| byte.is_ascii_control()) {
		return Err(
			ProxyError::SubstrateEgressFailed(
				"credential provider returned an unusable secret".to_owned(),
			)
			.into(),
		);
	}
	Ok(secret.to_vec())
}

async fn fetch_policy(
	target: &SimpleBackendReferenceWithPolicies,
	client: &PolicyClient,
	identity: &ActorIdentity,
) -> Result<protos::ateapi::EgressPolicy, ProxyError> {
	let policy_client =
		client.with_outbound(OutboundCallKind::Policy, OutboundCallSubtype::Substrate);
	let channel = target.grpc_channel(policy_client.clone());
	let mut control = protos::ateapi::control_client::ControlClient::new(channel);
	let mut request = tonic::Request::new(protos::ateapi::GetActorEgressPolicyRequest {
		name: "default".to_owned(),
		actor: Some(protos::ateapi::ObjectRef {
			atespace: identity.atespace.clone(),
			name: identity.actor_name.clone(),
		}),
	});
	let mut span = policy_client.start_grpc_span(
		&mut request,
		target.target.as_ref(),
		"/substrate.ateapi.v1.Control/GetActorEgressPolicy",
	);
	let response = crate::proxy::dtrace::scope_future(
		Some(TRACE_POLICY_KIND),
		control.get_actor_egress_policy(request),
	)
	.await;
	if let Some(span) = span.as_deref_mut() {
		span.record_grpc_result(&response);
	}
	drop(span);
	match response {
		Ok(response) => Ok(response.into_inner()),
		Err(status) if matches!(status.code(), Code::Unavailable | Code::DeadlineExceeded) => Err(
			ProxyError::SubstrateEgressUnavailable(format!("actor egress policy unavailable: {status}")),
		),
		Err(status) => Err(ProxyError::SubstrateEgressDenied(format!(
			"actor egress policy denied: {status}"
		))),
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EgressTlsMode {
	Intercept,
	Passthrough,
}

pub(crate) async fn authorize_tls(
	client: &PolicyClient,
	connection: &mut Extension,
	sni: &str,
) -> Result<Option<EgressTlsMode>, ProxyError> {
	let Some(identity) = connection.get::<ActorIdentity>() else {
		return Ok(None);
	};
	let policy = connection.get::<EgressActorResolution>().ok_or_else(|| {
		ProxyError::SubstrateEgressDenied("missing actor egress control API".to_owned())
	})?;
	let port = connection
		.get::<TCPConnectionInfo>()
		.expect("tcp connection must be set")
		.local_addr
		.port();
	let policy = fetch_policy(&policy.target, client, identity).await?;
	let mode = tls_mode(&policy, sni, port)?;
	if mode == EgressTlsMode::Passthrough {
		connection.insert(DynamicBackendOverride(Target::from((sni, port))));
	}
	Ok(Some(mode))
}

fn tls_mode(
	policy: &protos::ateapi::EgressPolicy,
	sni: &str,
	port: u16,
) -> Result<EgressTlsMode, ProxyError> {
	let sni = sni.strip_suffix('.').unwrap_or(sni).to_ascii_lowercase();
	if !valid_hostname(&sni) {
		return Err(ProxyError::SubstrateEgressDenied(
			"missing or invalid TLS SNI".to_owned(),
		));
	}
	let Some(rule) = matching_destination_rule(policy, &sni, port, MatchPhase::ClientHello) else {
		return Err(ProxyError::SubstrateEgressDenied(
			"actor egress policy denied TLS destination".to_owned(),
		));
	};
	Ok(if rule.https.is_some() {
		EgressTlsMode::Intercept
	} else {
		EgressTlsMode::Passthrough
	})
}

/// The address an actor's CONNECT named, set only when it named an IP literal,
/// and the gateway's own address the CONNECT arrived on. A tcp rule decides on
/// the first; the second is one of the gateway's addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EgressConnectAddress {
	pub target: SocketAddr,
	pub gateway: IpAddr,
}

/// Authorizes a stream that is neither TLS nor HTTP against the actor's tcp
/// rules, on the address and port its CONNECT named, and pins the backend to
/// that address. A hostname in a rule matches when the gateway resolves it to
/// that address.
pub(crate) async fn authorize_tcp(
	client: &PolicyClient,
	connection: &mut Extension,
) -> Result<(), ProxyError> {
	let identity = connection
		.get::<ActorIdentity>()
		.ok_or_else(|| ProxyError::SubstrateEgressDenied("missing actor identity".to_owned()))?;
	let resolution = connection.get::<EgressActorResolution>().ok_or_else(|| {
		ProxyError::SubstrateEgressDenied("missing actor egress control API".to_owned())
	})?;
	let Some(EgressConnectAddress {
		target: address,
		gateway,
	}) = connection.get::<EgressConnectAddress>().copied()
	else {
		return Err(ProxyError::SubstrateEgressDenied(
			"tcp egress needs a CONNECT to an IP address".to_owned(),
		));
	};
	if !tcp_destination_routable(address.ip()) {
		return Err(ProxyError::SubstrateEgressDenied(format!(
			"tcp egress to {address} names no single host"
		)));
	}
	let gateway = gateway_addresses(gateway)?;
	let policy = fetch_policy(&resolution.target, client, identity).await?;
	let upstream = &client.inputs.upstream;
	if !tcp_rule_allows(&policy, address, &gateway, |name| {
		upstream.resolve_all(name.into())
	})
	.await
	{
		return Err(ProxyError::SubstrateEgressDenied(format!(
			"actor egress policy denied tcp destination {address}"
		)));
	}
	connection.insert(DynamicBackendOverride(Target::Address(address)));
	Ok(())
}

/// Reports whether an address names a single host. Unspecified, multicast and
/// broadcast addresses never do, whatever a rule names.
fn tcp_destination_routable(ip: IpAddr) -> bool {
	match ip.to_canonical() {
		IpAddr::V4(ip) => !(ip.is_unspecified() || ip.is_multicast() || ip.is_broadcast()),
		IpAddr::V6(ip) => !(ip.is_unspecified() || ip.is_multicast()),
	}
}

/// Ranges whose addresses are the gateway's or its node's: loopback,
/// link-local (the cloud metadata service at 169.254.169.254 among them), and
/// the metadata and platform services clouds serve elsewhere (AWS's IPv6
/// metadata service, Alibaba Cloud's, Azure's WireServer), and the local-use
/// NAT64 prefix, whose embedded addresses a gateway cannot tell apart.
const TCP_LOCAL_RANGES: [&str; 8] = [
	"127.0.0.0/8",
	"169.254.0.0/16",
	"100.100.100.200/32",
	"168.63.129.16/32",
	"::1/128",
	"fe80::/10",
	"fd00:ec2::254/128",
	"64:ff9b:1::/48",
];

/// Every address of the gateway, canonical: the one the CONNECT arrived on and
/// those of its network interfaces, so that a dual-stack pod's address of the
/// other family is the gateway's too. Unlistable interfaces refuse the stream.
fn gateway_addresses(arrived_on: IpAddr) -> Result<Vec<IpAddr>, ProxyError> {
	let mut addresses = interface_addresses().map_err(|error| {
		ProxyError::SubstrateEgressDenied(format!("cannot list the gateway's addresses: {error}"))
	})?;
	addresses.push(arrived_on);
	for address in &mut addresses {
		*address = address.to_canonical();
	}
	addresses.sort_unstable();
	addresses.dedup();
	Ok(addresses)
}

#[cfg(target_family = "unix")]
fn interface_addresses() -> std::io::Result<Vec<IpAddr>> {
	Ok(
		nix::ifaddrs::getifaddrs()?
			.filter_map(|interface| interface.address)
			.filter_map(|address| {
				address
					.as_sockaddr_in()
					.map(|v4| IpAddr::V4(v4.ip()))
					.or_else(|| address.as_sockaddr_in6().map(|v6| IpAddr::V6(v6.ip())))
			})
			.collect(),
	)
}

#[cfg(not(target_family = "unix"))]
fn interface_addresses() -> std::io::Result<Vec<IpAddr>> {
	Ok(Vec::new())
}

/// The range of TCP_LOCAL_RANGES an address is in, or one of the gateway's own
/// addresses as a single-address range. Only a CIDR inside that range reaches
/// it: a wider CIDR or a hostname that resolves to it never does. An address
/// of the NAT64 well-known prefix (64:ff9b::/96) is local as a whole when the
/// IPv4 address it embeds is.
fn tcp_local_range(ip: IpAddr, gateway: &[IpAddr]) -> Option<ipnet::IpNet> {
	if let IpAddr::V6(v6) = ip
		&& let [0x0064, 0xff9b, 0, 0, 0, 0, high, low] = v6.segments()
	{
		let embedded = IpAddr::V4(std::net::Ipv4Addr::from(
			(u32::from(high) << 16) | u32::from(low),
		));
		return tcp_local_range(embedded, gateway).map(|_| ipnet::IpNet::from(ip));
	}
	if gateway.contains(&ip) {
		return Some(ipnet::IpNet::from(ip));
	}
	TCP_LOCAL_RANGES
		.iter()
		.map(|range| range.parse::<ipnet::IpNet>().expect("valid range"))
		.find(|range| range.contains(&ip))
}

/// Reports whether a tcp rule on the address's port names a CIDR that
/// contains it, or a hostname that resolves to it. Names are resolved only
/// when no CIDR decides.
async fn tcp_rule_allows<F, Fut>(
	policy: &protos::ateapi::EgressPolicy,
	address: SocketAddr,
	gateway: &[IpAddr],
	resolve: F,
) -> bool
where
	F: Fn(&str) -> Fut,
	Fut: std::future::Future<Output = Box<[IpAddr]>>,
{
	let ip = address.ip().to_canonical();
	let local = tcp_local_range(ip, gateway);
	let mut names = Vec::new();
	for rule in &policy.rules {
		// These fields form an API union. An invalid union cannot authorize traffic.
		let (None, None, None, Some(tcp)) = (&rule.http, &rule.https, &rule.tls_passthrough, &rule.tcp)
		else {
			continue;
		};
		if port_rank(tcp.ports.as_ref(), address.port(), 0).is_none() {
			continue;
		}
		if tcp.cidrs.iter().any(|cidr| {
			cidr.parse::<ipnet::IpNet>().is_ok_and(|net| {
				net.trunc() == net && net.contains(&ip) && local.is_none_or(|range| range.contains(&net))
			})
		}) {
			return true;
		}
		if local.is_none() {
			names.extend(
				tcp
					.hostnames
					.iter()
					.map(String::as_str)
					.filter(|name| valid_hostname(name)),
			);
		}
	}
	names.sort_unstable();
	names.dedup();
	futures::future::join_all(names.into_iter().map(resolve))
		.await
		.iter()
		.any(|addresses| {
			addresses
				.iter()
				.any(|resolved| resolved.to_canonical() == ip)
		})
}

#[derive(Clone, Copy)]
enum MatchPhase {
	HttpRequest,
	HttpsRequest,
	ClientHello,
}

fn matching_rule<'a>(
	policy: &'a protos::ateapi::EgressPolicy,
	req: &Request,
) -> Result<&'a protos::ateapi::EgressRule, ProxyResponse> {
	let destination = req
		.extensions()
		.get::<cel::DestinationContext>()
		.ok_or_else(|| {
			ProxyError::SubstrateEgressDenied("missing egress destination context".to_owned())
		})?;
	let protocol = req
		.extensions()
		.get::<EgressRequestProtocol>()
		.ok_or_else(|| {
			ProxyError::SubstrateEgressDenied("missing egress protocol context".to_owned())
		})?;
	let phase = match protocol {
		EgressRequestProtocol::Http => MatchPhase::HttpRequest,
		EgressRequestProtocol::Https => MatchPhase::HttpsRequest,
	};
	matching_destination_rule(
		policy,
		destination.hostname.as_deref().unwrap_or_default(),
		destination.port,
		phase,
	)
	.ok_or_else(|| {
		ProxyError::SubstrateEgressDenied("actor egress policy denied destination".to_owned()).into()
	})
}

fn matching_destination_rule<'a>(
	policy: &'a protos::ateapi::EgressPolicy,
	hostname: &str,
	port: u16,
	phase: MatchPhase,
) -> Option<&'a protos::ateapi::EgressRule> {
	let mut best = None;
	for rule in &policy.rules {
		// These fields form an API union. An invalid union cannot authorize traffic.
		let (hostnames, ports, default_port) = match (
			phase,
			&rule.http,
			&rule.https,
			&rule.tls_passthrough,
			&rule.tcp,
		) {
			(MatchPhase::HttpRequest, Some(http), None, None, None) => {
				(&http.hostnames, http.ports.as_ref(), 80)
			},
			(MatchPhase::HttpsRequest | MatchPhase::ClientHello, None, Some(https), None, None) => {
				(&https.hostnames, https.ports.as_ref(), 443)
			},
			(MatchPhase::ClientHello, None, None, Some(tls), None) => {
				(&tls.hostnames, tls.ports.as_ref(), 0)
			},
			_ => continue,
		};
		let Some(port_rank) = port_rank(ports, port, default_port) else {
			continue;
		};
		let Some(name_rank) = hostnames
			.iter()
			.filter_map(|pattern| hostname_rank(pattern, hostname))
			.min()
		else {
			continue;
		};
		let rank = (name_rank, port_rank);
		if best.is_none_or(|(_, best_rank)| rank < best_rank) {
			best = Some((rule, rank));
		}
	}
	best.map(|(rule, _)| rule)
}

fn port_rank(ports: Option<&protos::ateapi::Ports>, port: u16, default_port: u16) -> Option<u8> {
	if port == 0 {
		return None;
	}
	let Some(ports) = ports else {
		return (port == default_port).then_some(0);
	};
	if ports.all.is_some() {
		return ports.numbers.is_empty().then_some(1);
	}
	if ports
		.numbers
		.iter()
		.any(|number| !(1..=65535).contains(number))
	{
		return None;
	}
	ports.numbers.contains(&i32::from(port)).then_some(0)
}

fn hostname_rank(pattern: &str, hostname: &str) -> Option<u8> {
	if pattern == "*" {
		let host = hostname
			.strip_prefix('[')
			.and_then(|host| host.strip_suffix(']'))
			.unwrap_or(hostname);
		return (valid_hostname(hostname) || host.parse::<std::net::IpAddr>().is_ok()).then_some(2);
	}
	if !valid_hostname(hostname) {
		return None;
	}
	if let Some(suffix) = pattern.strip_prefix("*.") {
		if !valid_hostname(suffix) {
			return None;
		}
		let label = hostname.strip_suffix(suffix)?.strip_suffix('.')?;
		(!label.is_empty() && !label.contains('.')).then_some(1)
	} else {
		(pattern == hostname).then_some(0)
	}
}

fn valid_hostname(hostname: &str) -> bool {
	hostname.len() <= 253
		&& hostname.split('.').all(super::valid_resource_name)
		&& !hostname
			.rsplit('.')
			.next()
			.unwrap_or_default()
			.bytes()
			.all(|byte| byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
	use std::net::IpAddr;

	use super::*;

	fn request(protocol: EgressRequestProtocol, hostname: &str, port: u16) -> Request {
		let mut request = Request::new(crate::http::Body::empty());
		request.extensions_mut().insert(cel::DestinationContext {
			address: "192.0.2.1".parse::<IpAddr>().unwrap(),
			port,
			hostname: Some(hostname.into()),
		});
		request.extensions_mut().insert(protocol);
		request
	}

	fn rule(
		protocol: EgressRequestProtocol,
		hostnames: &[&str],
		ports: Option<protos::ateapi::Ports>,
	) -> protos::ateapi::EgressRule {
		let hostnames = hostnames.iter().map(|name| (*name).to_owned()).collect();
		match protocol {
			EgressRequestProtocol::Http => protos::ateapi::EgressRule {
				http: Some(protos::ateapi::HttpRule {
					hostnames,
					ports,
					effects: None,
				}),
				..Default::default()
			},
			EgressRequestProtocol::Https => protos::ateapi::EgressRule {
				https: Some(protos::ateapi::HttpsRule {
					hostnames,
					ports,
					effects: None,
				}),
				..Default::default()
			},
		}
	}

	fn all_ports() -> Option<protos::ateapi::Ports> {
		Some(protos::ateapi::Ports {
			all: Some(protos::ateapi::AllPorts {}),
			numbers: vec![],
		})
	}

	fn ports(numbers: &[i32]) -> Option<protos::ateapi::Ports> {
		Some(protos::ateapi::Ports {
			all: None,
			numbers: numbers.to_vec(),
		})
	}

	#[test]
	fn rules_are_protocol_specific_and_default_ports_are_enforced() {
		for (protocol, other, default_port) in [
			(
				EgressRequestProtocol::Http,
				EgressRequestProtocol::Https,
				80,
			),
			(
				EgressRequestProtocol::Https,
				EgressRequestProtocol::Http,
				443,
			),
		] {
			let policy = protos::ateapi::EgressPolicy {
				rules: vec![rule(protocol, &["api.example.com"], None)],
				..Default::default()
			};
			assert!(matching_rule(&policy, &request(protocol, "api.example.com", default_port)).is_ok());
			assert!(matching_rule(&policy, &request(other, "api.example.com", default_port)).is_err());
			assert!(matching_rule(&policy, &request(protocol, "api.example.com", 8443)).is_err());
			assert!(
				matching_rule(
					&policy,
					&request(protocol, "other.example.com", default_port)
				)
				.is_err()
			);
		}
	}

	#[test]
	fn explicit_and_all_ports() {
		for (selected_ports, port, allowed) in [
			(ports(&[80, 8080]), 8080, true),
			(ports(&[80, 8080]), 443, false),
			(all_ports(), 8443, true),
			(all_ports(), 0, false),
			(ports(&[]), 80, false),
			(ports(&[-1, 80]), 80, false),
			(ports(&[65536, 80]), 80, false),
			(
				Some(protos::ateapi::Ports {
					all: Some(protos::ateapi::AllPorts {}),
					numbers: vec![80],
				}),
				80,
				false,
			),
		] {
			let policy = protos::ateapi::EgressPolicy {
				rules: vec![rule(EgressRequestProtocol::Http, &["*"], selected_ports)],
				..Default::default()
			};
			assert_eq!(
				matching_rule(
					&policy,
					&request(EgressRequestProtocol::Http, "api.example.com", port)
				)
				.is_ok(),
				allowed
			);
		}
	}

	#[test]
	fn hostnames_match_exact_single_label_and_any_patterns() {
		for (pattern, hostname, expected) in [
			("api.example.com", "api.example.com", Some(0)),
			("api.example.com", "other.example.com", None),
			("*.example.com", "api.example.com", Some(1)),
			("*.example.com", "nested.api.example.com", None),
			("*.example.com", "example.com", None),
			("*.example.com", ".example.com", None),
			("*", "api.example.com", Some(2)),
			("*", "192.0.2.1", Some(2)),
			("*", "[2001:db8::1]", Some(2)),
			("192.0.2.1", "192.0.2.1", None),
			("*.2.1", "192.0.2.1", None),
			("*", "", None),
			("*", "foo..example.com", None),
			("*", "example.com.", None),
			("*", "example.com:80", None),
			("*", "-invalid.example", None),
			("*", "01.2.3.4", None),
			("*", "unicode.Kom", None),
			("*", "user@example.com", None),
		] {
			assert_eq!(
				hostname_rank(pattern, hostname),
				expected,
				"{pattern}: {hostname}"
			);
		}
	}

	#[test]
	fn most_specific_rule_wins_independent_of_order() {
		let protocol = EgressRequestProtocol::Https;
		// Name specificity precedes port specificity. Match the best pattern in each rule.
		let ranked = [
			rule(
				protocol,
				&["unrelated.example", "api.example.com"],
				ports(&[443]),
			),
			rule(protocol, &["api.example.com"], all_ports()),
			rule(protocol, &["*.example.com"], ports(&[443])),
			rule(protocol, &["*.example.com"], all_ports()),
			rule(protocol, &["*"], ports(&[443])),
			rule(protocol, &["*"], all_ports()),
		];
		for pair in ranked.windows(2) {
			for rules in [
				vec![pair[0].clone(), pair[1].clone()],
				vec![pair[1].clone(), pair[0].clone()],
			] {
				let policy = protos::ateapi::EgressPolicy {
					rules,
					..Default::default()
				};
				assert_eq!(
					matching_rule(&policy, &request(protocol, "api.example.com", 443)).unwrap(),
					&pair[0]
				);
			}
		}
	}

	#[test]
	fn passthrough_empty_and_invalid_rules_cannot_authorize_http_requests() {
		let passthrough = protos::ateapi::EgressRule {
			tls_passthrough: Some(protos::ateapi::TlsPassthroughRule {
				hostnames: vec!["*".to_owned()],
				ports: all_ports(),
			}),
			..Default::default()
		};
		let mut invalid = rule(EgressRequestProtocol::Http, &["*"], all_ports());
		invalid.https = rule(EgressRequestProtocol::Https, &["*"], all_ports()).https;
		for rules in [
			vec![],
			vec![Default::default()],
			vec![passthrough],
			vec![invalid],
		] {
			let policy = protos::ateapi::EgressPolicy {
				rules,
				..Default::default()
			};
			for protocol in [EgressRequestProtocol::Http, EgressRequestProtocol::Https] {
				assert!(matching_rule(&policy, &request(protocol, "api.example.com", 443)).is_err());
			}
		}
	}

	#[test]
	fn missing_transport_or_destination_denies() {
		let policy = protos::ateapi::EgressPolicy {
			rules: vec![rule(EgressRequestProtocol::Http, &["*"], all_ports())],
			..Default::default()
		};
		let mut req = request(EgressRequestProtocol::Http, "api.example.com", 80);
		req.extensions_mut().remove::<EgressRequestProtocol>();
		assert!(matching_rule(&policy, &req).is_err());
		req.extensions_mut().insert(EgressRequestProtocol::Http);
		req.extensions_mut().remove::<cel::DestinationContext>();
		assert!(matching_rule(&policy, &req).is_err());
	}

	fn passthrough(
		hostnames: &[&str],
		ports: Option<protos::ateapi::Ports>,
	) -> protos::ateapi::EgressRule {
		protos::ateapi::EgressRule {
			tls_passthrough: Some(protos::ateapi::TlsPassthroughRule {
				hostnames: hostnames.iter().map(|name| (*name).to_owned()).collect(),
				ports,
			}),
			..Default::default()
		}
	}

	#[test]
	fn tls_selection_compares_https_and_passthrough_specificity() {
		for (https, tls, expected) in [
			(
				rule(
					EgressRequestProtocol::Https,
					&["api.example.com"],
					all_ports(),
				),
				passthrough(&["*.example.com"], ports(&[443])),
				EgressTlsMode::Intercept,
			),
			(
				rule(
					EgressRequestProtocol::Https,
					&["*.example.com"],
					ports(&[443]),
				),
				passthrough(&["api.example.com"], all_ports()),
				EgressTlsMode::Passthrough,
			),
			(
				rule(EgressRequestProtocol::Https, &["api.example.com"], None),
				passthrough(&["api.example.com"], all_ports()),
				EgressTlsMode::Intercept,
			),
			(
				rule(
					EgressRequestProtocol::Https,
					&["api.example.com"],
					all_ports(),
				),
				passthrough(&["api.example.com"], ports(&[443])),
				EgressTlsMode::Passthrough,
			),
		] {
			for rules in [vec![https.clone(), tls.clone()], vec![tls, https]] {
				let policy = protos::ateapi::EgressPolicy {
					rules,
					..Default::default()
				};
				assert_eq!(
					tls_mode(&policy, "API.Example.com.", 443).unwrap(),
					expected
				);
			}
		}
	}

	#[test]
	fn tls_rejects_invalid_sni_and_unmatched_destinations() {
		let policy = protos::ateapi::EgressPolicy {
			rules: vec![passthrough(&["*"], ports(&[443]))],
			..Default::default()
		};
		for sni in ["", "127.0.0.1", "bad..example", "example.com.."] {
			assert!(tls_mode(&policy, sni, 443).is_err(), "{sni}");
		}
		assert!(matches!(
			tls_mode(&policy, "api.example.com", 8443),
			Err(ProxyError::SubstrateEgressDenied(_))
		));
		for rules in [
			vec![],
			vec![passthrough(&["*"], None)],
			vec![rule(EgressRequestProtocol::Http, &["*"], all_ports())],
		] {
			let policy = protos::ateapi::EgressPolicy {
				rules,
				..Default::default()
			};
			assert!(matches!(
				tls_mode(&policy, "api.example.com", 443),
				Err(ProxyError::SubstrateEgressDenied(_))
			));
		}
	}

	#[test]
	fn credential_uri_uses_the_exact_authority_as_provider_name() {
		assert_eq!(
			provider_name("ate-secret://kubernetes.io/default/token"),
			Some("kubernetes.io")
		);
		assert_eq!(provider_name("https://kubernetes.io/default/token"), None);
		assert_eq!(
			provider_name("substrate-secret://kubernetes.io/default/token"),
			None
		);
		assert_eq!(provider_name("ate-secret:///default/token"), None);
		assert_eq!(provider_name("ate-secret://kubernetes.io:443/token"), None);
	}

	#[test]
	fn credential_header_overwrites_with_a_sensitive_prefixed_secret() {
		let (name, value) = credential_header(
			&protos::ateapi::CredentialHeader {
				header: "authorization".to_owned(),
				prefix: "Bearer ".to_owned(),
				credential_uri: "ate-secret://kubernetes.io/default/token".to_owned(),
				..Default::default()
			},
			b"token\n".to_vec(),
		)
		.unwrap();
		assert_eq!(name, ::http::header::AUTHORIZATION);
		assert_eq!(value, "Bearer token");
		assert!(value.is_sensitive());
	}

	#[test]
	fn credential_headers_cannot_modify_routing_or_framing_headers() {
		for header in [
			"host",
			"content-length",
			"connection",
			"keep-alive",
			"proxy-authenticate",
			"proxy-authorization",
			"te",
			"trailer",
			"transfer-encoding",
			"upgrade",
		] {
			let injection = protos::ateapi::CredentialHeader {
				header: header.to_owned(),
				prefix: String::new(),
				credential_uri: "ate-secret://kubernetes.io/default/token".to_owned(),
				..Default::default()
			};
			assert!(credential_header(&injection, b"token".to_vec()).is_err());
		}
	}

	#[test]
	fn credential_provider_errors_answer_by_class() {
		use ::http::StatusCode;
		for (code, want) in [
			(Code::NotFound, StatusCode::FORBIDDEN),
			(Code::PermissionDenied, StatusCode::FORBIDDEN),
			(Code::FailedPrecondition, StatusCode::FORBIDDEN),
			(Code::InvalidArgument, StatusCode::FORBIDDEN),
			(Code::Unavailable, StatusCode::SERVICE_UNAVAILABLE),
			(Code::DeadlineExceeded, StatusCode::SERVICE_UNAVAILABLE),
			(Code::ResourceExhausted, StatusCode::SERVICE_UNAVAILABLE),
			(Code::Unauthenticated, StatusCode::BAD_GATEWAY),
			(Code::Internal, StatusCode::BAD_GATEWAY),
			(Code::Unknown, StatusCode::BAD_GATEWAY),
			(Code::Unimplemented, StatusCode::BAD_GATEWAY),
		] {
			let response = credential_provider_error(
				"ate-secret://kubernetes.io/default/token",
				tonic::Status::new(code, "provider failed"),
			)
			.into_response_with_grpc(false);
			assert_eq!(response.status(), want, "{code:?}");
		}
	}

	#[test]
	fn path_prefixes_match_at_segment_boundaries() {
		for (prefix, path, want) in [
			("/api", "/api", true),
			("/api", "/api/v4", true),
			("/api", "/apis", false),
			("/api/", "/api/v4", true),
			("/api/", "/api/", true),
			("/api/", "/api", false),
			("/", "/anything", true),
			("/api/v4", "/api", false),
		] {
			assert_eq!(
				path_prefix_matches(prefix, path),
				want,
				"path_prefix_matches({prefix:?}, {path:?})"
			);
		}
	}

	#[test]
	fn replace_headers_apply_the_longest_matching_path_prefix() {
		fn header(name: &str, uri: &str, path_prefix: &str) -> protos::ateapi::CredentialHeader {
			protos::ateapi::CredentialHeader {
				header: name.to_owned(),
				credential_uri: uri.to_owned(),
				path_prefix: path_prefix.to_owned(),
				..Default::default()
			}
		}
		let bearer = header("authorization", "ate-secret://p/bearer", "/api/");
		let basic = header("Authorization", "ate-secret://p/basic", "");
		let api_key = header("X-Api-Key", "ate-secret://p/key", "/api/");
		let v4 = header("AUTHORIZATION", "ate-secret://p/v4", "/api/v4");
		let replace_headers = [bearer, basic, api_key, v4];
		let uris = |path: &str| -> Vec<&str> {
			replace_headers_for(&replace_headers, path)
				.into_iter()
				.map(|injection| injection.credential_uri.as_str())
				.collect()
		};

		let basic = "ate-secret://p/basic";
		let bearer = "ate-secret://p/bearer";
		let api_key = "ate-secret://p/key";
		let v4 = "ate-secret://p/v4";
		for (path, want) in [
			("/repo.git/info/refs", vec![basic]),
			("/api/users", vec![bearer, api_key]),
			("/api/v4/projects", vec![v4, api_key]),
			("/api/v4", vec![v4, api_key]),
			("/api/v40", vec![bearer, api_key]),
			("/api", vec![basic]),
			// Paths an origin could resolve elsewhere get only the entry without a prefix.
			("/api/../admin", vec![basic]),
			("/api/v4/..;/x", vec![basic]),
			("/api//v4", vec![basic]),
			("/api/%2e%2e/admin", vec![basic]),
			("/api/%2Fv4", vec![basic]),
			("/api\\v4", vec![basic]),
			("", vec![basic]),
		] {
			assert_eq!(uris(path), want, "replace_headers_for({path:?})");
		}

		// Without a prefixed entry, every path gets the unprefixed one, as before path prefixes.
		let unprefixed = [header("authorization", basic, "")];
		for path in ["/", "/api/v4", "/api/../admin"] {
			let chosen = replace_headers_for(&unprefixed, path);
			assert_eq!(chosen.len(), 1, "{path:?}");
			assert_eq!(chosen[0].credential_uri, basic);
		}
		// A header whose only entry is scoped elsewhere is not replaced at all.
		let scoped = [header("authorization", bearer, "/api/")];
		assert!(replace_headers_for(&scoped, "/repo.git").is_empty());
		assert_eq!(replace_headers_for(&scoped, "/api/v4").len(), 1);
	}

	#[test]
	fn replace_headers_ignore_a_prefix_a_path_could_escape() {
		let codex = protos::ateapi::CredentialHeader {
			header: "authorization".to_owned(),
			prefix: "Bearer ".to_owned(),
			credential_uri: "ate-secret://p/codex".to_owned(),
			path_prefix: "/backend-api/codex/".to_owned(),
			..Default::default()
		};
		let host = protos::ateapi::CredentialHeader {
			header: "authorization".to_owned(),
			prefix: "Basic ".to_owned(),
			credential_uri: "ate-secret://p/host".to_owned(),
			..Default::default()
		};
		let scoped = [codex.clone()];
		let with_host = [codex, host];
		// The egress passes the request path without its query.
		let path = |target: &'static str| target.split('?').next().unwrap_or_default();

		// The prefix itself still applies under it.
		let chosen = replace_headers_for(&with_host, path("/backend-api/codex/responses?stream=1"));
		assert_eq!(chosen.len(), 1);
		assert_eq!(chosen[0].credential_uri, "ate-secret://p/codex");

		for target in [
			"/backend-api/codex/../../other",
			"/backend-api/codex/..",
			"/backend-api/codex/./../other",
			"/backend-api/codex/%2e%2e/%2e%2e/other",
			"/backend-api/codex/%2E%2E/%2E%2E/other",
			"/backend-api/codex/%2e%2E/other",
			"/backend-api/codex/.%2e/other",
			"/backend-api/codex/%2e./other",
			"/backend-api/codex/..%2fother",
			"/backend-api/codex/..%2Fother",
			"/backend-api/codex/%2fother",
			"/backend-api/codex%2f..%2f..%2fother",
			"/backend-api/codex//../../other",
			"/backend-api/codex//other",
			"/backend-api/codex/..;/..;/other",
			"/backend-api/codex/..;x=1/other",
			"/backend-api/codex/a%2f..%2f..%2fother",
			"/backend-api/codex/a%2F.%2Fother",
			"/backend-api/codex/a%2F%2Fother",
			"/backend-api/codex/a%2F..;x=1%2Fother",
			r"/backend-api/codex/..\..\other",
			"/backend-api/codex/..%5c..%5cother",
			"/backend-api/codex/..%5C..%5Cother",
			"/backend-api/codex/../../other?q=/backend-api/codex/",
		] {
			assert!(
				replace_headers_for(&scoped, path(target)).is_empty(),
				"replace_headers_for(scoped, {target:?}) applies the scoped entry"
			);
			let chosen = replace_headers_for(&with_host, path(target));
			assert_eq!(
				chosen.len(),
				1,
				"replace_headers_for(with_host, {target:?})"
			);
			assert_eq!(
				chosen[0].credential_uri, "ate-secret://p/host",
				"replace_headers_for(with_host, {target:?})"
			);
		}
	}

	#[test]
	fn replace_headers_apply_a_prefix_to_an_encoded_slash_under_it() {
		let api = protos::ateapi::CredentialHeader {
			header: "authorization".to_owned(),
			prefix: "Bearer ".to_owned(),
			credential_uri: "ate-secret://p/gitlab-api".to_owned(),
			path_prefix: "/api/v4/".to_owned(),
			..Default::default()
		};
		let scoped = [api];
		// GitLab addresses a project, a file and a branch by its URL-encoded path.
		for path in [
			"/api/v4/projects/workspaces%2Fworkspace-proof",
			"/api/v4/projects/workspaces%2fworkspace-proof/merge_requests",
			"/api/v4/projects/group%2Fsub%2Fproject/repository/files/docs%2FREADME.md/raw",
			"/api/v4/projects/group%2Fproject/repository/branches/feature%252Fx",
		] {
			let chosen = replace_headers_for(&scoped, path);
			assert_eq!(chosen.len(), 1, "replace_headers_for(scoped, {path:?})");
			assert_eq!(chosen[0].credential_uri, "ate-secret://p/gitlab-api");
		}
		// An encoded slash never moves a path under a prefix it is not under as written.
		for path in ["/api%2Fv4/projects", "/api/v4%2Fprojects", "/api%2fv4/user"] {
			assert!(
				replace_headers_for(&scoped, path).is_empty(),
				"replace_headers_for(scoped, {path:?}) applies the scoped entry"
			);
		}
	}

	#[test]
	fn malformed_credential_secrets_fail_closed() {
		let injection = protos::ateapi::CredentialHeader {
			header: "authorization".to_owned(),
			prefix: "Bearer ".to_owned(),
			credential_uri: "ate-secret://kubernetes.io/default/token".to_owned(),
			..Default::default()
		};
		for secret in [Vec::new(), b"bad\nsecret".to_vec()] {
			let response = credential_header(&injection, secret)
				.unwrap_err()
				.downcast()
				.into_response_with_grpc(false);
			assert_eq!(response.status(), ::http::StatusCode::BAD_GATEWAY);
		}
	}

	#[test]
	fn credential_cache_reuses_fresh_entries_and_expires_stale_ones() {
		let cache = CredentialCache::new(16);
		let key = CredentialCacheKey {
			actor_identity: "spiffe://substrate-actor.local/actor/default/example".to_owned(),
			actor_uid: Some("uid-1".to_owned()),
			uri: "ate-secret://kubernetes.io/default/token".to_owned(),
		};
		let now = Instant::now();
		cache.insert(key.clone(), b"token".to_vec(), now);
		assert_eq!(
			cache.get(&key, now, DEFAULT_CREDENTIAL_CACHE_TTL),
			Some(b"token".to_vec())
		);

		let stale_at = now - DEFAULT_CREDENTIAL_CACHE_TTL - Duration::from_secs(1);
		cache.insert(key.clone(), b"stale".to_vec(), stale_at);
		assert_eq!(cache.get(&key, now, DEFAULT_CREDENTIAL_CACHE_TTL), None);
	}

	#[test]
	fn credential_cache_is_partitioned_by_actor_identity_uid_and_uri() {
		let cache = CredentialCache::new(16);
		let now = Instant::now();
		let key = CredentialCacheKey {
			actor_identity: "spiffe://substrate-actor.local/actor/default/one".to_owned(),
			actor_uid: Some("uid-1".to_owned()),
			uri: "ate-secret://kubernetes.io/default/token".to_owned(),
		};
		cache.insert(key.clone(), b"one".to_vec(), now);

		let another_actor = CredentialCacheKey {
			actor_identity: "spiffe://substrate-actor.local/actor/default/two".to_owned(),
			..key.clone()
		};
		let another_uri = CredentialCacheKey {
			uri: "ate-secret://kubernetes.io/default/other".to_owned(),
			..key.clone()
		};
		let another_uid = CredentialCacheKey {
			actor_uid: Some("uid-2".to_owned()),
			..key
		};
		assert_eq!(
			cache.get(&another_actor, now, DEFAULT_CREDENTIAL_CACHE_TTL),
			None
		);
		assert_eq!(
			cache.get(&another_uri, now, DEFAULT_CREDENTIAL_CACHE_TTL),
			None
		);
		assert_eq!(
			cache.get(&another_uid, now, DEFAULT_CREDENTIAL_CACHE_TTL),
			None
		);
	}

	#[test]
	fn credential_providers_accept_inline_backends_with_policies() {
		let provider: CredentialProvider = serde_json::from_value(serde_json::json!({
			"uriAuthority": "kubernetes.io",
			"target": {
				"host": "https://credprovider.example.test:50051",
				"policies": { "backendTLS": {} }
			}
		}))
		.unwrap();
		assert_eq!(provider.uri_authority, "kubernetes.io");
		assert!(matches!(
			provider.target.target.as_ref(),
			crate::types::agent::SimpleBackendReference::InlineBackend(_)
		));
	}

	fn tcp_rule(
		hostnames: &[&str],
		cidrs: &[&str],
		ports: Option<protos::ateapi::Ports>,
	) -> protos::ateapi::EgressRule {
		protos::ateapi::EgressRule {
			tcp: Some(protos::ateapi::TcpRule {
				hostnames: hostnames.iter().map(|name| (*name).to_owned()).collect(),
				cidrs: cidrs.iter().map(|cidr| (*cidr).to_owned()).collect(),
				ports,
			}),
			..Default::default()
		}
	}

	// A dual-stack gateway: one address of each family.
	const GATEWAY: [IpAddr; 2] = [
		IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 5)),
		IpAddr::V6(std::net::Ipv6Addr::new(0xfd00, 0x10, 0, 0, 0, 0, 0, 5)),
	];

	async fn tcp_allows(rules: Vec<protos::ateapi::EgressRule>, address: &str) -> bool {
		let policy = protos::ateapi::EgressPolicy {
			rules,
			..Default::default()
		};
		tcp_rule_allows(&policy, address.parse().unwrap(), &GATEWAY, |name| {
			let addresses: Box<[IpAddr]> = match name {
				"git.example.com" => [
					"192.0.2.10".parse().unwrap(),
					"2001:db8::10".parse().unwrap(),
				]
				.into(),
				"rebind.example.com" => [
					"127.0.0.1".parse().unwrap(),
					"169.254.169.254".parse().unwrap(),
					"10.0.0.5".parse().unwrap(),
				]
				.into(),
				_ => [].into(),
			};
			async move { addresses }
		})
		.await
	}

	#[tokio::test]
	async fn tcp_rules_match_cidrs_and_resolved_hostnames_on_their_ports() {
		for (rule, address, allowed) in [
			(
				tcp_rule(&[], &["192.0.2.0/24"], ports(&[22])),
				"192.0.2.7:22",
				true,
			),
			(
				tcp_rule(&[], &["192.0.2.0/24"], ports(&[22])),
				"192.0.2.7:2222",
				false,
			),
			(
				tcp_rule(&[], &["192.0.2.0/24"], ports(&[22])),
				"198.51.100.7:22",
				false,
			),
			(
				tcp_rule(&[], &["192.0.2.0/24"], all_ports()),
				"192.0.2.7:5432",
				true,
			),
			(
				tcp_rule(&[], &["2001:db8::/32"], ports(&[22])),
				"[2001:db8::1]:22",
				true,
			),
			(
				tcp_rule(&[], &["192.0.2.0/24"], ports(&[22])),
				"[::ffff:192.0.2.7]:22",
				true,
			),
			// A CIDR with host bits set is invalid and authorizes nothing.
			(
				tcp_rule(&[], &["192.0.2.7/24"], ports(&[22])),
				"192.0.2.7:22",
				false,
			),
			(
				tcp_rule(&[], &["not-a-cidr"], ports(&[22])),
				"192.0.2.7:22",
				false,
			),
			(
				tcp_rule(&["git.example.com"], &[], ports(&[22])),
				"192.0.2.10:22",
				true,
			),
			(
				tcp_rule(&["git.example.com"], &[], ports(&[22])),
				"[2001:db8::10]:22",
				true,
			),
			(
				tcp_rule(&["git.example.com"], &[], ports(&[22])),
				"192.0.2.11:22",
				false,
			),
			(
				tcp_rule(&["git.example.com"], &[], ports(&[22])),
				"192.0.2.10:443",
				false,
			),
			(
				tcp_rule(&["unknown.example.com"], &[], ports(&[22])),
				"192.0.2.10:22",
				false,
			),
			// Ports are required: a tcp rule has no default port.
			(
				tcp_rule(&[], &["192.0.2.0/24"], None),
				"192.0.2.7:22",
				false,
			),
		] {
			assert_eq!(tcp_allows(vec![rule], address).await, allowed, "{address}");
		}
	}

	#[tokio::test]
	async fn tcp_reaches_local_addresses_only_through_a_cidr_inside_them() {
		for (rule, address, allowed) in [
			(
				tcp_rule(&[], &["0.0.0.0/0"], all_ports()),
				"127.0.0.1:15000",
				false,
			),
			(
				tcp_rule(&[], &["0.0.0.0/0"], all_ports()),
				"169.254.169.254:80",
				false,
			),
			(tcp_rule(&[], &["::/0"], all_ports()), "[::1]:15000", false),
			(
				tcp_rule(&[], &["::/0"], all_ports()),
				"[::ffff:127.0.0.1]:15000",
				false,
			),
			(
				tcp_rule(&[], &["::/0"], all_ports()),
				"[fd00:ec2::254]:80",
				false,
			),
			(
				tcp_rule(&[], &["100.64.0.0/10"], all_ports()),
				"100.100.100.200:80",
				false,
			),
			(
				tcp_rule(&[], &["168.63.0.0/16"], all_ports()),
				"168.63.129.16:80",
				false,
			),
			(
				tcp_rule(&["rebind.example.com"], &[], all_ports()),
				"127.0.0.1:15000",
				false,
			),
			(
				tcp_rule(&["rebind.example.com"], &[], all_ports()),
				"169.254.169.254:80",
				false,
			),
			(
				tcp_rule(&[], &["127.0.0.1/32"], all_ports()),
				"127.0.0.1:15000",
				true,
			),
			(
				tcp_rule(&[], &["169.254.0.0/16"], all_ports()),
				"169.254.169.254:80",
				true,
			),
			(
				tcp_rule(&[], &["fd00:ec2::254/128"], all_ports()),
				"[fd00:ec2::254]:80",
				true,
			),
			// The gateway's own address, among its peers.
			(
				tcp_rule(&[], &["10.0.0.0/8"], all_ports()),
				"10.0.0.5:15000",
				false,
			),
			(
				tcp_rule(&["rebind.example.com"], &[], all_ports()),
				"10.0.0.5:15000",
				false,
			),
			(
				tcp_rule(&[], &["10.0.0.0/8"], all_ports()),
				"10.0.0.6:15000",
				true,
			),
			(
				tcp_rule(&[], &["10.0.0.5/32"], all_ports()),
				"10.0.0.5:15000",
				true,
			),
			// Its address of each family, whichever the CONNECT arrived on.
			(
				tcp_rule(&[], &["0.0.0.0/0"], all_ports()),
				"10.0.0.5:15000",
				false,
			),
			(
				tcp_rule(&[], &["::/0"], all_ports()),
				"[fd00:10::5]:15000",
				false,
			),
			(
				tcp_rule(&[], &["fd00:10::/64"], all_ports()),
				"[fd00:10::5]:15000",
				false,
			),
			(
				tcp_rule(&[], &["fd00:10::/64"], all_ports()),
				"[fd00:10::6]:15000",
				true,
			),
			(
				tcp_rule(&[], &["fd00:10::5/128"], all_ports()),
				"[fd00:10::5]:15000",
				true,
			),
			(
				tcp_rule(&["rebind.example.com"], &[], all_ports()),
				"[fd00:10::5]:15000",
				false,
			),
			// NAT64 forms of local addresses.
			(
				tcp_rule(&[], &["64:ff9b::/96"], all_ports()),
				"[64:ff9b::7f00:1]:15000",
				false,
			),
			(
				tcp_rule(&[], &["64:ff9b::/96"], all_ports()),
				"[64:ff9b::a9fe:a9fe]:80",
				false,
			),
			(
				tcp_rule(&[], &["64:ff9b::/96"], all_ports()),
				"[64:ff9b::c000:207]:22",
				true,
			),
			(
				tcp_rule(&[], &["64:ff9b::7f00:1/128"], all_ports()),
				"[64:ff9b::7f00:1]:15000",
				true,
			),
			(
				tcp_rule(&[], &["::/0"], all_ports()),
				"[64:ff9b:1:7f00:1::]:15000",
				false,
			),
			// The gateway's IPv4 address in its NAT64 form.
			(
				tcp_rule(&[], &["::/0"], all_ports()),
				"[64:ff9b::a00:5]:15000",
				false,
			),
			(
				tcp_rule(&[], &["64:ff9b::/96"], all_ports()),
				"[64:ff9b::a00:5]:15000",
				false,
			),
			(
				tcp_rule(&[], &["64:ff9b::a00:5/128"], all_ports()),
				"[64:ff9b::a00:5]:15000",
				true,
			),
			(
				tcp_rule(&[], &["64:ff9b::/96"], all_ports()),
				"[64:ff9b::a00:6]:15000",
				true,
			),
		] {
			assert_eq!(tcp_allows(vec![rule], address).await, allowed, "{address}");
		}
	}

	#[test]
	fn gateway_addresses_hold_the_connect_address_and_every_interface_address() {
		let addresses = gateway_addresses("::ffff:192.0.2.99".parse().unwrap()).unwrap();
		assert!(addresses.contains(&"192.0.2.99".parse().unwrap()));
		let interfaces = interface_addresses().unwrap();
		#[cfg(target_family = "unix")]
		assert!(!interfaces.is_empty(), "no interface address listed");
		for interface in interfaces {
			assert!(addresses.contains(&interface.to_canonical()), "{interface}");
		}
	}

	#[tokio::test]
	async fn tcp_reaches_no_interface_address_through_a_wider_cidr() {
		// The CONNECT arrived on loopback; every other address of the host,
		// of either family, is the gateway's all the same.
		let gateway = gateway_addresses(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)).unwrap();
		let policy = protos::ateapi::EgressPolicy {
			rules: vec![tcp_rule(&[], &["0.0.0.0/0", "::/0"], all_ports())],
			..Default::default()
		};
		for ip in &gateway {
			let address = SocketAddr::new(*ip, 15000);
			let allowed = tcp_rule_allows(&policy, address, &gateway, |_| async {
				Box::<[IpAddr]>::default()
			})
			.await;
			assert!(!allowed, "{address}");
		}
	}

	#[tokio::test]
	async fn tcp_ignores_other_protocols_and_invalid_unions() {
		let mut both = tcp_rule(&[], &["192.0.2.0/24"], ports(&[22]));
		both.http = Some(protos::ateapi::HttpRule {
			hostnames: vec!["*".to_owned()],
			ports: ports(&[22]),
			effects: None,
		});
		assert!(!tcp_allows(vec![both], "192.0.2.7:22").await);
		assert!(
			!tcp_allows(
				vec![rule(EgressRequestProtocol::Http, &["*"], all_ports())],
				"192.0.2.7:22"
			)
			.await
		);
		// A tcp rule never decides a request or a ClientHello.
		let policy = protos::ateapi::EgressPolicy {
			rules: vec![tcp_rule(&["git.example.com"], &[], all_ports())],
			..Default::default()
		};
		assert!(tls_mode(&policy, "git.example.com", 22).is_err());
		assert!(
			matching_rule(
				&policy,
				&request(EgressRequestProtocol::Http, "git.example.com", 22)
			)
			.is_err()
		);
	}

	#[test]
	fn tcp_destinations_name_a_single_host() {
		for (address, routable) in [
			("192.0.2.7", true),
			("127.0.0.1", true),
			("0.0.0.0", false),
			("255.255.255.255", false),
			("224.0.0.1", false),
			("::", false),
			("ff02::1", false),
			("::ffff:0.0.0.0", false),
		] {
			assert_eq!(
				tcp_destination_routable(address.parse().unwrap()),
				routable,
				"{address}"
			);
		}
	}
}

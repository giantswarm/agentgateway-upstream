// Inspired by https://github.com/cdriehuys/axum-jwks/blob/main/axum-jwks/src/jwks.rs (MIT license)
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;

use ::cel::types::dynamic::DynamicType;
use jsonwebtoken::jwk::{AlgorithmParameters, EllipticCurve, JwkSet, KeyAlgorithm};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use secrecy::SecretString;
use serde_json::{Map, Value};

use crate::http::Request;
use crate::http::auth::AuthorizationLocation;
use crate::proxy::dtrace::{self};
use crate::telemetry::log::RequestLog;
use crate::*;

#[cfg(test)]
#[path = "jwt_tests.rs"]
mod tests;

/// Fixtures the JWT tests and the xDS conversion tests share.
#[cfg(test)]
pub(crate) mod test_support {
	use std::future::Future;
	use std::pin::Pin;
	use std::sync::Mutex;
	use std::sync::atomic::{AtomicUsize, Ordering};

	use serde_json::json;

	use super::JwksRefreshSource;

	// One ed25519 key serves every kid, so one signature verifies against any of them:
	// the tests are about kid lookup and refetching, not distinct key material.
	pub(crate) const ED25519_PRIVATE_KEY: &[u8] = &[
		0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20,
		0x6a, 0xc3, 0xfd, 0xee, 0xee, 0x29, 0x8a, 0x92, 0x63, 0x8b, 0x70, 0x0c, 0x4b, 0x11, 0x7c, 0xc3,
		0x2e, 0x2d, 0x2a, 0xce, 0x0d, 0xfd, 0x78, 0x76, 0x94, 0xe2, 0x4c, 0xae, 0x8a, 0xd5, 0x82, 0x34,
	];
	pub(crate) const ED25519_PUBLIC_X: &str = "2-Jj2UvNCvQiUPNYRgSi0cJSPiJI6Rs6D0UTeEpQVj8";

	pub(crate) fn jwks_with_kids(kids: &[&str]) -> serde_json::Value {
		json!({
			"keys": kids
				.iter()
				.map(|kid| json!({
					"use": "sig",
					"kty": "OKP",
					"kid": kid,
					"crv": "Ed25519",
					"x": ED25519_PUBLIC_X,
				}))
				.collect::<Vec<_>>()
		})
	}

	pub(crate) fn build_token(kid: &str, issuer: &str, aud: &str, exp: u64) -> String {
		let claims = json!({ "iss": issuer, "aud": aud, "sub": "test-user", "exp": exp });
		let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::EdDSA);
		header.kid = Some(kid.to_string());
		jsonwebtoken::encode(
			&header,
			&claims,
			&jsonwebtoken::EncodingKey::from_ed_der(ED25519_PRIVATE_KEY),
		)
		.unwrap()
	}

	/// A control plane that answers every refresh with one JWKS document and records
	/// what it was asked.
	#[derive(Debug)]
	pub(crate) struct FakeJwksRefreshSource {
		jwks: String,
		calls: AtomicUsize,
		asked: Mutex<Vec<(String, String)>>,
	}

	impl FakeJwksRefreshSource {
		pub(crate) fn new(jwks: serde_json::Value) -> Self {
			Self {
				jwks: jwks.to_string(),
				calls: AtomicUsize::new(0),
				asked: Mutex::new(Vec::new()),
			}
		}

		pub(crate) fn calls(&self) -> usize {
			self.calls.load(Ordering::SeqCst)
		}

		pub(crate) fn asked(&self) -> Vec<(String, String)> {
			self.asked.lock().unwrap().clone()
		}
	}

	impl JwksRefreshSource for FakeJwksRefreshSource {
		fn refresh<'a>(
			&'a self,
			key: &'a str,
			kid: &'a str,
		) -> Pin<Box<dyn Future<Output = Option<String>> + Send + 'a>> {
			Box::pin(async move {
				self.calls.fetch_add(1, Ordering::SeqCst);
				self
					.asked
					.lock()
					.unwrap()
					.push((key.to_owned(), kid.to_owned()));
				Some(self.jwks.clone())
			})
		}
	}
}

const TRACE_POLICY_KIND: &str = "jwt";

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum TokenError {
	#[error("the token is invalid or malformed: {0:?}")]
	Invalid(jsonwebtoken::errors::Error),

	#[error("the token header is malformed: {0:?}")]
	InvalidHeader(jsonwebtoken::errors::Error),

	#[error("no bearer token found")]
	Missing,

	#[error("the token header does not specify a `kid`")]
	MissingKeyId,

	#[error("token uses the unknown key {0:?}")]
	UnknownKeyId(String),

	#[error("failed to strip validated credentials from the request: {0}")]
	CredentialRemoval(String),
}

#[derive(thiserror::Error, Debug)]
pub enum JwkError {
	#[error("failed to load JWKS: {0}")]
	JwkLoadError(anyhow::Error),
	#[error("failed to parse JWKS: {0}")]
	JwksParseError(#[from] serde_json::Error),
	#[error("the key is missing the `kid` attribute")]
	MissingKeyId,
	#[error("could not construct a decoding key for {key_id:?}: {error:?}")]
	DecodingError {
		key_id: String,
		error: jsonwebtoken::errors::Error,
	},
	#[error(
		"the key {key_id:?} uses an unsupported algorithm {algorithm:?} (supported: RSA, EC, OKP[Ed25519])"
	)]
	UnexpectedAlgorithm {
		algorithm: AlgorithmParameters,
		key_id: String,
	},
	#[error("the key {key_id:?} uses unsupported OKP curve {curve:?} (supported: Ed25519)")]
	UnsupportedCurve {
		key_id: String,
		curve: EllipticCurve,
	},
}

#[derive(Clone)]
pub struct Jwt {
	mode: Mode,
	providers: Vec<Provider>,
	location: AuthorizationLocation,
	preserve_token: bool,
	// Only remote JWKS sources get a refresher.
	refreshers: Vec<JwksRefresher>,
}

#[derive(Clone)]
pub struct Provider {
	issuer: String,
	keys: HashMap<String, Jwk>,
}

/// A source the data plane asks for a provider's current JWKS document outside the
/// schedule: the control plane that fetched and pushed the keys inline (Kubernetes),
/// over the connection the data plane holds to it already. The source bounds its own
/// calls per key.
pub trait JwksRefreshSource: Send + Sync + std::fmt::Debug {
	/// The JWKS document the source holds for `key` after the call (within the source's
	/// interval, the one it answered in it), or `None` when the call failed or timed out.
	fn refresh<'a>(
		&'a self,
		key: &'a str,
		kid: &'a str,
	) -> Pin<Box<dyn Future<Output = Option<String>> + Send + 'a>>;
}

pub type JwksRefreshSourceRef = Arc<dyn JwksRefreshSource>;

/// Rebuilds a single provider's keys on demand, so a token whose `kid` predates a
/// key rotation can succeed without waiting for the next scheduled refresh.
#[derive(Clone)]
pub(crate) struct JwksRefresher {
	keys: JwksRefreshKeys,
	issuer: String,
	audiences: Option<Vec<String>>,
	jwt_validation_options: JWTValidationOptions,
}

/// Where a refresher gets the provider's current JWKS document from.
#[derive(Clone)]
enum JwksRefreshKeys {
	/// A remote source this process fetches itself (local configuration).
	Remote {
		manager: crate::resource_manager::ResourceManager,
		resource: crate::resource_manager::ResourceRef,
	},
	/// Keys the control plane fetched and pushed inline under `key`.
	ControlPlane {
		source: JwksRefreshSourceRef,
		key: String,
	},
}

impl JwksRefresher {
	/// A refresher for keys the control plane pushed inline under `key`.
	pub(crate) fn control_plane(
		source: JwksRefreshSourceRef,
		key: String,
		issuer: String,
		audiences: Option<Vec<String>>,
		jwt_validation_options: JWTValidationOptions,
	) -> Self {
		Self {
			keys: JwksRefreshKeys::ControlPlane { source, key },
			issuer,
			audiences,
			jwt_validation_options,
		}
	}

	/// Returns `None` when the refresh was debounced, failed, or the key is still missing.
	async fn refresh(&self, kid: &str) -> Option<Jwk> {
		let bytes: bytes::Bytes = match &self.keys {
			JwksRefreshKeys::Remote { manager, resource } => {
				match manager.refresh_and_wait(resource).await {
					Ok(bytes) => bytes,
					Err(error) => {
						debug!(%error, "on-demand JWKS refresh did not run");
						return None;
					},
				}
			},
			JwksRefreshKeys::ControlPlane { source, key } => source.refresh(key, kid).await?.into(),
		};
		let jwks: JwkSet = match serde_json::from_slice(&bytes) {
			Ok(jwks) => jwks,
			Err(error) => {
				warn!(%error, "refetched JWKS could not be parsed");
				return None;
			},
		};
		let provider = match Provider::from_jwks(
			jwks,
			self.issuer.clone(),
			self.audiences.clone(),
			self.jwt_validation_options.clone(),
		) {
			Ok(provider) => provider,
			Err(error) => {
				warn!(%error, "refetched JWKS has no usable keys");
				return None;
			},
		};
		provider.keys.get(kid).cloned()
	}
}

// TODO: can we give anything useful here?
impl serde::Serialize for Jwt {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		#[derive(serde::Serialize)]
		#[serde(rename_all = "camelCase")]
		pub struct Serde<'a> {
			mode: Mode,
			providers: &'a Vec<Provider>,
			location: &'a AuthorizationLocation,
			#[serde(default, skip_serializing_if = "std::ops::Not::not")]
			preserve_token: bool,
		}
		Serde {
			mode: self.mode,
			providers: &self.providers,
			location: &self.location,
			preserve_token: self.preserve_token,
		}
		.serialize(serializer)
	}
}

impl serde::Serialize for Provider {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		#[derive(serde::Serialize)]
		pub struct Serde<'a> {
			issuer: &'a str,
			keys: Vec<&'a str>,
		}
		Serde {
			issuer: &self.issuer,
			keys: self.keys.keys().map(|x| x.as_str()).collect::<Vec<_>>(),
		}
		.serialize(serializer)
	}
}

impl Debug for Jwt {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Jwt").finish()
	}
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(
	feature = "schema",
	schemars(untagged, deny_unknown_fields, rename_all_fields = "camelCase")
)]
pub enum LocalJwtConfig {
	/// Validate JWTs against one or more trusted token issuers.
	Multi {
		/// Controls whether requests must include a JWT and how validation failures are handled.
		#[cfg_attr(feature = "schema", schemars(default))]
		mode: Mode,
		/// Where to read the JWT from in incoming requests.
		#[cfg_attr(feature = "schema", schemars(default))]
		location: AuthorizationLocation,
		/// Keep a successfully validated JWT in its original location.
		#[cfg_attr(feature = "schema", schemars(default))]
		preserve_token: bool,
		/// Trusted issuers and their signing keys.
		providers: Vec<ProviderConfig>,
	},
	/// Validate JWTs against a single trusted token issuer.
	Single {
		/// Controls whether requests must include a JWT and how validation failures are handled.
		#[cfg_attr(feature = "schema", schemars(default))]
		mode: Mode,
		/// Where to read the JWT from in incoming requests.
		#[cfg_attr(feature = "schema", schemars(default))]
		location: AuthorizationLocation,
		/// Keep a successfully validated JWT in its original location.
		#[cfg_attr(feature = "schema", schemars(default))]
		preserve_token: bool,
		/// Expected token issuer. The JWT `iss` claim is required and must match.
		issuer: String,
		/// Accepted token audiences. A non-empty list requires a matching JWT `aud` claim.
		audiences: Option<Vec<String>>,
		/// JSON Web Key Set used to verify token signatures. Can be inline, from a file, or fetched remotely.
		jwks: serdes::FileInlineOrRemote,
		/// Claim requirements to enforce after the token signature is verified.
		#[cfg_attr(feature = "schema", schemars(default))]
		jwt_validation_options: JWTValidationOptions,
	},
}

#[apply(schema_de!)]
struct LocalJwtMultiConfig {
	#[serde(default)]
	mode: Mode,
	#[serde(default)]
	location: AuthorizationLocation,
	#[serde(default)]
	preserve_token: bool,
	providers: Vec<ProviderConfig>,
}

#[apply(schema_de!)]
struct LocalJwtSingleConfig {
	#[serde(default)]
	mode: Mode,
	#[serde(default)]
	location: AuthorizationLocation,
	#[serde(default)]
	preserve_token: bool,
	issuer: String,
	audiences: Option<Vec<String>>,
	jwks: serdes::FileInlineOrRemote,
	#[serde(default)]
	jwt_validation_options: JWTValidationOptions,
}

// Select the configuration shape before deserializing it so serde does not discard the
// actionable error from each arm of an untagged enum.
impl<'de> Deserialize<'de> for LocalJwtConfig {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let value = Value::deserialize(deserializer)?;
		if value.get("providers").is_some() {
			let config: LocalJwtMultiConfig =
				serde_json::from_value(value).map_err(serde::de::Error::custom)?;
			Ok(Self::Multi {
				mode: config.mode,
				location: config.location,
				preserve_token: config.preserve_token,
				providers: config.providers,
			})
		} else {
			let config: LocalJwtSingleConfig =
				serde_json::from_value(value).map_err(serde::de::Error::custom)?;
			Ok(Self::Single {
				mode: config.mode,
				location: config.location,
				preserve_token: config.preserve_token,
				issuer: config.issuer,
				audiences: config.audiences,
				jwks: config.jwks,
				jwt_validation_options: config.jwt_validation_options,
			})
		}
	}
}

#[apply(schema_de!)]
pub struct ProviderConfig {
	/// Expected token issuer. The JWT `iss` claim is required and must match.
	pub issuer: String,
	/// Accepted token audiences. A non-empty list requires a matching JWT `aud` claim.
	pub audiences: Option<Vec<String>>,
	/// JSON Web Key Set used to verify token signatures. Can be inline, from a file, or fetched remotely.
	pub jwks: serdes::FileInlineOrRemote,
	/// Claim requirements to enforce after the token signature is verified.
	#[serde(default)]
	pub jwt_validation_options: JWTValidationOptions,
}

#[apply(schema_enum!)]
#[derive(Default)]
pub enum Mode {
	/// Require a valid JWT from a configured issuer.
	Strict,
	/// Validate the JWT when present.
	/// This is the default option.
	/// Warning: this allows requests without a JWT.
	#[default]
	Optional,
	/// Decode valid JWTs for later policy use.
	/// Warning: this allows requests with missing or invalid JWTs.
	Permissive,
}

/// JWT validation options controlling which claims must be present in a token.
///
/// The `required_claims` set specifies which RFC 7519 registered claims must
/// exist in the token payload before validation proceeds. Only the following
/// values are recognized: `exp`, `nbf`, `aud`, `iss`, `sub`. Other registered
/// claims such as `iat` and `jti` are **not** enforced by the underlying
/// `jsonwebtoken` library and will be silently ignored.
///
/// This only enforces **presence**. A configured issuer and non-empty audience
/// list independently require `iss` and `aud`, respectively. Standard claims
/// like `exp` and `nbf` have their values validated independently (e.g., expiry
/// is always checked when the `exp` claim is present, regardless of this setting).
///
/// Defaults to `["exp"]`.
#[derive(Eq, PartialEq)]
#[apply(schema_de!)]
pub struct JWTValidationOptions {
	/// Claims that must be present in the token before validation.
	/// Only "exp", "nbf", "aud", "iss", "sub" are enforced; others
	/// (including "iat" and "jti") are ignored.
	/// Defaults to ["exp"]. Use an empty list to add no claim requirements beyond
	/// those implied by the configured issuer and audiences.
	#[serde(default = "default_required_claims")]
	pub required_claims: HashSet<String>,
}

fn default_required_claims() -> HashSet<String> {
	HashSet::from(["exp".to_owned()])
}

/// The only claim names the jsonwebtoken library actually enforces.
const SUPPORTED_REQUIRED_CLAIMS: &[&str] = &["exp", "nbf", "aud", "iss", "sub"];

/// Log a warning for each claim in `required_claims` that the library silently ignores.
fn warn_unsupported_claims(required_claims: &HashSet<String>) {
	for claim in required_claims {
		if !SUPPORTED_REQUIRED_CLAIMS.contains(&claim.as_str()) {
			tracing::warn!(
				claim = %claim,
				supported = ?SUPPORTED_REQUIRED_CLAIMS,
				"ignoring unrecognized required claim"
			);
		}
	}
}

impl Default for JWTValidationOptions {
	fn default() -> Self {
		Self {
			required_claims: default_required_claims(),
		}
	}
}

impl LocalJwtConfig {
	pub async fn try_into(
		self,
		resources: &crate::resource_manager::ResourceFetcher,
	) -> Result<Jwt, JwkError> {
		let (mode, authorization_location, preserve_token, providers_cfg) = match self {
			LocalJwtConfig::Multi {
				mode,
				location: authorization_location,
				preserve_token,
				providers,
			} => (mode, authorization_location, preserve_token, providers),
			LocalJwtConfig::Single {
				mode,
				location: authorization_location,
				preserve_token,
				issuer,
				audiences,
				jwks,
				jwt_validation_options,
			} => (
				mode,
				authorization_location,
				preserve_token,
				vec![ProviderConfig {
					issuer,
					audiences,
					jwks,
					jwt_validation_options,
				}],
			),
		};

		let mut providers = Vec::with_capacity(providers_cfg.len());
		let mut refreshers = Vec::new();
		for pc in providers_cfg {
			let remote_resource = match pc
				.jwks
				.as_resource_ref(crate::resource_manager::ResourceKind::Jwks)
			{
				Some(resource @ crate::resource_manager::ResourceRef::Http { .. }) => Some(resource),
				_ => None,
			};
			let jwks: JwkSet = pc
				.jwks
				.load::<JwkSet>(resources, crate::resource_manager::ResourceKind::Jwks)
				.await
				.map_err(JwkError::JwkLoadError)?;
			let provider = Provider::from_jwks(
				jwks,
				pc.issuer.clone(),
				pc.audiences.clone(),
				pc.jwt_validation_options.clone(),
			)?;
			if let (Some(resource), Some(manager)) =
				(remote_resource, resources.managed_resource_manager())
			{
				refreshers.push(JwksRefresher {
					keys: JwksRefreshKeys::Remote { manager, resource },
					issuer: pc.issuer,
					audiences: pc.audiences,
					jwt_validation_options: pc.jwt_validation_options,
				});
			}
			providers.push(provider);
		}
		Ok(Jwt {
			mode,
			providers,
			location: authorization_location,
			preserve_token,
			refreshers,
		})
	}
}

impl Provider {
	pub fn from_jwks(
		jwks: JwkSet,
		issuer: String,
		audiences: Option<Vec<String>>,
		jwt_validation_options: JWTValidationOptions,
	) -> Result<Provider, JwkError> {
		warn_unsupported_claims(&jwt_validation_options.required_claims);

		let mut keys = HashMap::new();
		let to_supported_alg = |key_algorithm: Option<KeyAlgorithm>| match key_algorithm {
			Some(key_alg) => jsonwebtoken::Algorithm::from_str(key_alg.to_string().as_str()).ok(),
			_ => None,
		};

		for jwk in jwks.keys {
			let kid = jwk.common.key_id.ok_or(JwkError::MissingKeyId)?;

			let decoding_key =
				match &jwk.algorithm {
					AlgorithmParameters::RSA(rsa) => DecodingKey::from_rsa_components(&rsa.n, &rsa.e)
						.map_err(|err| JwkError::DecodingError {
							key_id: kid.clone(),
							error: err,
						})?,
					AlgorithmParameters::EllipticCurve(ec) => DecodingKey::from_ec_components(&ec.x, &ec.y)
						.map_err(|err| JwkError::DecodingError {
						key_id: kid.clone(),
						error: err,
					})?,
					AlgorithmParameters::OctetKeyPair(okp) => match &okp.curve {
						EllipticCurve::Ed25519 => {
							DecodingKey::from_ed_components(&okp.x).map_err(|err| JwkError::DecodingError {
								key_id: kid.clone(),
								error: err,
							})?
						},
						other => {
							return Err(JwkError::UnsupportedCurve {
								key_id: kid,
								curve: other.clone(),
							});
						},
					},
					other => {
						return Err(JwkError::UnexpectedAlgorithm {
							key_id: kid,
							algorithm: other.to_owned(),
						});
					},
				};

			let supported_algorithms = match to_supported_alg(jwk.common.key_algorithm) {
				None => {
					// If they did not explicitly set the key algorithm, which is optional, then we can infer it
					// based on the algorithm properties.
					// Add each key algorithm in the correct family.
					match &jwk.algorithm {
						AlgorithmParameters::EllipticCurve(_) => {
							vec![Algorithm::ES256, Algorithm::ES384]
						},
						AlgorithmParameters::RSA(_) => {
							vec![
								Algorithm::RS256,
								Algorithm::RS384,
								Algorithm::RS512,
								Algorithm::PS256,
								Algorithm::PS384,
								Algorithm::PS512,
							]
						},
						AlgorithmParameters::OctetKeyPair(_) => {
							vec![Algorithm::EdDSA]
						},
						_ => unreachable!(),
					}
				},
				Some(explicit_alg) => {
					vec![explicit_alg]
				},
			};
			// The new() requires 1 algorithm, so just pass the first before we override it
			let mut validation = Validation::new(*supported_algorithms.first().unwrap());
			validation.validate_nbf = true;
			validation.algorithms = supported_algorithms;
			// Override required_spec_claims with the user-configured set. A configured
			// issuer or audience also implies that the corresponding claim must exist;
			// otherwise there is nothing to match against the configured value.
			// validate_exp remains true, so exp is still validated if present.
			validation.required_spec_claims = jwt_validation_options.required_claims.clone();
			validation.set_issuer(std::slice::from_ref(&issuer));
			validation.required_spec_claims.insert("iss".to_owned());
			if let Some(audiences) = audiences.as_ref().filter(|audiences| !audiences.is_empty()) {
				validation.set_audience(audiences);
				validation.required_spec_claims.insert("aud".to_owned());
			} else {
				validation.validate_aud = false;
			}

			keys.insert(
				kid,
				Jwk {
					decoding: decoding_key,
					validation,
				},
			);
		}

		Ok(Provider { issuer, keys })
	}
}

impl Jwt {
	pub fn from_providers(
		providers: Vec<Provider>,
		mode: Mode,
		authorization_location: AuthorizationLocation,
		preserve_token: bool,
	) -> Jwt {
		Jwt {
			mode,
			providers,
			location: authorization_location,
			preserve_token,
			refreshers: Vec::new(),
		}
	}

	/// Attaches the on-demand refreshers of providers whose keys the control plane
	/// pushed inline; built from configuration a provider's own remote source is wired
	/// in [`LocalJwtConfig::try_into`] instead.
	pub(crate) fn with_refreshers(mut self, refreshers: Vec<JwksRefresher>) -> Jwt {
		self.refreshers = refreshers;
		self
	}
}

#[derive(Clone)]
struct Jwk {
	decoding: DecodingKey,
	validation: Validation,
}

#[derive(serde::Deserialize)]
struct UnverifiedIssuer {
	iss: Option<String>,
}

fn decode_with_key(token: &str, key: &Jwk) -> Result<Claims, TokenError> {
	let decoded_token =
		decode::<Map<String, Value>>(token, &key.decoding, &key.validation).map_err(|error| {
			debug!(?error, "Token is malformed or does not pass validation.");

			TokenError::Invalid(error)
		})?;

	Ok(Claims {
		inner: decoded_token.claims,
		jwt: SecretString::new(token.into()),
	})
}

#[derive(Clone, Debug, Default)]
pub struct Claims {
	pub inner: Map<String, Value>,
	pub jwt: SecretString,
}

#[cfg(feature = "schema")]
impl schemars::JsonSchema for Claims {
	fn schema_name() -> std::borrow::Cow<'static, str> {
		"JwtClaims".into()
	}

	fn json_schema(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
		schemars::json_schema!({
			"type": "object",
			"properties": {
				"rawToken": {
					"type": "string",
					"description": "The raw bearer token. Redacted by default; use `jwt.rawToken.unredacted()` to access the actual value."
				}
			},
			"additionalProperties": true
		})
	}
}

impl DynamicType for Claims {
	fn materialize(&self) -> cel::Value<'_> {
		self.inner.materialize()
	}

	fn field(&self, field: &str) -> Option<cel::Value<'_>> {
		match field {
			"rawToken" => Some(crate::cel::secret_string_to_value(&self.jwt)),
			_ => self.inner.field(field),
		}
	}
}

impl Serialize for Claims {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.inner.serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for Claims {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let inner = Map::deserialize(deserializer)?;
		Ok(Claims {
			inner,
			jwt: SecretString::new("".into()),
		})
	}
}

impl Jwt {
	pub fn expressions(&self) -> impl Iterator<Item = &crate::cel::Expression> {
		self.location.expression().into_iter()
	}

	pub async fn apply(
		&self,
		log: Option<&mut RequestLog>,
		req: &mut Request,
	) -> Result<(), TokenError> {
		let Some(token) = self.location.extract(req) else {
			// In strict mode, we require a token
			if self.mode == Mode::Strict {
				dtrace::pol_result!(
					dtrace::Error,
					Apply,
					"rejected request because JWT is required but missing"
				);
				return Err(TokenError::Missing);
			}
			// Otherwise with no, don't attempt to authenticate.
			dtrace::pol_result!(
				dtrace::Info,
				Skip,
				"request has no bearer token and JWT mode is not strict"
			);
			return Ok(());
		};
		let claims = match self.validate_claims_with_refresh(&token).await {
			Ok(claims) => claims,
			Err(e) if self.mode == Mode::Permissive => {
				dtrace::pol_result!(
					dtrace::Warn,
					Skip,
					"token verification failed ({e}), continue due to permissive mode"
				);
				return Ok(());
			},
			Err(e) => {
				dtrace::pol_result!(
					dtrace::Severity::Error,
					Apply,
					"rejected request because JWT validation failed: {e}"
				);
				return Err(e);
			},
		};

		if let Some(serde_json::Value::String(sub)) = claims.inner.get("sub")
			&& let Some(log) = log
		{
			log.jwt_sub = Some(sub.to_string());
		};
		if !self.preserve_token {
			self
				.location
				.remove(req)
				.map_err(|e| TokenError::CredentialRemoval(e.to_string()))?;
		}
		// Insert the claims into extensions so we can reference it later
		dtrace::pol_result!(
			dtrace::Severity::Info,
			Apply,
			"authenticated request with JWT claims {}",
			serde_json::to_string(&claims).unwrap_or_else(|_| "invalid claims".to_string())
		);
		req.extensions_mut().insert(claims);
		Ok(())
	}

	pub fn validate_claims(&self, token: &str) -> Result<Claims, TokenError> {
		let header = decode_header(token).map_err(|error| {
			debug!(?error, "Received token with invalid header.");

			TokenError::InvalidHeader(error)
		})?;
		let kid = header.kid.as_ref().ok_or_else(|| {
			debug!(?header, "Header is missing the `kid` attribute.");

			TokenError::MissingKeyId
		})?;

		let decode_with = |key: &Jwk| {
			decode::<Map<String, Value>>(token, &key.decoding, &key.validation).map(|decoded_token| {
				Claims {
					inner: decoded_token.claims,
					jwt: SecretString::new(token.into()),
				}
			})
		};

		// A kid is only unique within one issuer's JWKS, so different issuers can share a kid (Entra tenants share keys; unrelated IdPs can collide)
		// Only the provider(s) whose configured issuer has the same iss claim as the token and whose JWKS has the same kid are tried.
		// iss is read before verification to choose which providers to try. Once chosen, the iss, aud, exp, and signature are checked.
		let iss = jsonwebtoken::dangerous::insecure_decode_claims::<UnverifiedIssuer>(token)
			.ok()
			.and_then(|claims| claims.iss);

		let mut first_error = None;
		for provider in &self.providers {
			if iss.as_deref() != Some(provider.issuer.as_str()) {
				continue;
			}
			let Some(key) = provider.keys.get(kid) else {
				continue;
			};
			match decode_with(key) {
				Ok(claims) => return Ok(claims),
				Err(error) => {
					debug!(?error, issuer = %provider.issuer, "Token is malformed or does not pass validation.");
					first_error.get_or_insert(error);
				},
			}
		}
		if let Some(error) = first_error {
			return Err(TokenError::Invalid(error));
		}

		// No provider has both the token's iss and kid.
		// Covers: unknown issuer, iss missing or not a string, and iss matches but kid doesn't.
		// Falls back to the original, first provider that has the kid so that the same errors are produced.
		let key = self
			.providers
			.iter()
			.find_map(|provider| provider.keys.get(kid))
			.ok_or_else(|| {
				debug!(%kid, "Token refers to an unknown key.");

				TokenError::UnknownKeyId(kid.to_owned())
			})?;

		decode_with_key(token, key)
	}

	/// Validates a token, retrying once against a freshly fetched JWKS when
	/// the key id is unknown. Only providers backed by a remote JWKS source
	/// have a refresher attached, so this is a no-op elsewhere.
	async fn validate_claims_with_refresh(&self, token: &str) -> Result<Claims, TokenError> {
		match self.validate_claims(token) {
			Err(TokenError::UnknownKeyId(kid)) => {
				for refresher in &self.refreshers {
					if let Some(key) = refresher.refresh(&kid).await {
						return decode_with_key(token, &key);
					}
				}
				Err(TokenError::UnknownKeyId(kid))
			},
			result => result,
		}
	}
}

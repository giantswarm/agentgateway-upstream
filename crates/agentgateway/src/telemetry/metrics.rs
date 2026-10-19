use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::{Duration, Instant};

use agent_core::metrics::{
	CustomField, DefaultedUnknown, EncodeArc, EncodeDebug, EncodeDisplay, MetricRegistry,
	OptionallyEncode,
};
use agent_core::strng::RichStrng;
use agent_core::version;
use frozen_collections::FzHashSet;
use parking_lot::Mutex;
use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::counter;
use prometheus_client::metrics::family::{Family, MetricConstructor};
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::histogram::{Histogram as PromHistogram, NativeHistogramConfig};
use prometheus_client::metrics::info::Info;
use prometheus_client::registry::{Metric, Unit};
use tracing::{debug, trace};

use crate::HistogramMode;
use crate::http::substrate::ateattr::{ResumeDisposition, RouteOutcome};
use crate::mcp::MCPOperation;
use crate::proxy::ProxyResponseReason;
use crate::types::agent::TransportProtocol;

#[derive(Clone, Hash, Default, Debug, PartialEq, Eq, EncodeLabelSet)]
pub struct RouteIdentifier {
	pub bind: DefaultedUnknown<RichStrng>,
	pub gateway: DefaultedUnknown<RichStrng>,
	pub listener: DefaultedUnknown<RichStrng>,
	pub route: DefaultedUnknown<RichStrng>,
	pub route_rule: DefaultedUnknown<RichStrng>,
}

#[derive(
	Copy, Clone, Hash, Debug, PartialEq, Eq, prometheus_client::encoding::EncodeLabelValue, Default,
)]
pub enum GuardrailPhase {
	#[default]
	Request,
	Response,
}

#[derive(
	Copy,
	Clone,
	Hash,
	Debug,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	prometheus_client::encoding::EncodeLabelValue,
	Default,
)]
// Ordered by severity so streaming guards can retain the strongest window result.
pub enum GuardrailAction {
	#[default]
	Allow,
	FailOpen,
	/// Guard ran in observe mode: the verdict was recorded but not enforced.
	Audit,
	Mask,
	Reject,
}

#[derive(Clone, Hash, Default, Debug, PartialEq, Eq, EncodeLabelSet)]
pub struct GuardrailLabels {
	pub phase: GuardrailPhase,
	pub action: GuardrailAction,
}

#[derive(Clone, Hash, Default, Debug, PartialEq, Eq, EncodeLabelSet)]
pub struct MinimalHTTPLabels {
	pub backend: DefaultedUnknown<RichStrng>,

	#[prometheus(flatten)]
	pub route: RouteIdentifier,

	#[prometheus(flatten)]
	pub custom: CustomField,
}

impl From<HTTPLabels> for MinimalHTTPLabels {
	fn from(value: HTTPLabels) -> Self {
		Self {
			backend: value.backend,
			route: value.route,
			custom: value.custom,
		}
	}
}

#[derive(Clone, Hash, Default, Debug, PartialEq, Eq, EncodeLabelSet)]
pub struct HTTPLabels {
	pub backend: DefaultedUnknown<RichStrng>,
	pub protocol: DefaultedUnknown<EncodeDebug<crate::cel::BackendProtocol>>,

	pub method: DefaultedUnknown<EncodeDisplay<http::Method>>,
	pub status: DefaultedUnknown<EncodeDisplay<u16>>,
	pub reason: DefaultedUnknown<EncodeDisplay<ProxyResponseReason>>,

	#[prometheus(flatten)]
	pub route: RouteIdentifier,

	#[prometheus(flatten)]
	pub custom: CustomField,
}

#[derive(Clone, Hash, Default, Debug, PartialEq, Eq, EncodeLabelSet)]
pub struct GenAILabels {
	pub gen_ai_operation_name: DefaultedUnknown<RichStrng>,
	pub gen_ai_system: DefaultedUnknown<RichStrng>,
	pub gen_ai_request_model: DefaultedUnknown<RichStrng>,
	pub gen_ai_response_model: DefaultedUnknown<RichStrng>,

	#[prometheus(flatten)]
	pub route: RouteIdentifier,

	#[prometheus(flatten)]
	pub custom: CustomField,
}

#[derive(Clone, Hash, Default, Debug, PartialEq, Eq, EncodeLabelSet)]
pub struct GenAILabelsTokenUsage {
	pub gen_ai_token_type: DefaultedUnknown<RichStrng>,

	#[prometheus(flatten)]
	pub common: EncodeArc<GenAILabels>,
}

#[derive(Clone, Hash, Debug, PartialEq, Eq, EncodeLabelSet)]
pub struct ErrorTypeLabel {
	pub error_type: GenAIErrorType,
}

#[derive(Clone, Hash, Debug, PartialEq, Eq)]
pub enum GenAIErrorType {
	Other,
}

impl prometheus_client::encoding::EncodeLabelValue for GenAIErrorType {
	fn encode(
		&self,
		encoder: &mut prometheus_client::encoding::LabelValueEncoder,
	) -> Result<(), std::fmt::Error> {
		match self {
			Self::Other => "_OTHER".encode(encoder),
		}
	}
}

#[derive(Clone, Hash, Debug, PartialEq, Eq, EncodeLabelSet)]
pub struct GenAIRequestDurationLabels {
	#[prometheus(flatten)]
	pub common: EncodeArc<GenAILabels>,

	#[prometheus(flatten)]
	pub error: OptionallyEncode<ErrorTypeLabel>,
}

#[derive(Clone, Hash, Default, Debug, PartialEq, Eq, EncodeLabelSet)]
pub struct CostCatalogLookupLabels {
	pub status: crate::llm::catalog::CostLookupStatus,

	#[prometheus(flatten)]
	pub common: EncodeArc<GenAILabels>,
}

#[derive(Clone, Hash, Debug, PartialEq, Eq, EncodeLabelSet)]
pub struct MCPCall {
	pub method: DefaultedUnknown<RichStrng>,

	pub resource_type: DefaultedUnknown<MCPOperation>,
	pub server: DefaultedUnknown<RichStrng>,
	pub resource: DefaultedUnknown<RichStrng>,

	#[prometheus(flatten)]
	pub route: RouteIdentifier,

	#[prometheus(flatten)]
	pub custom: CustomField,
}

#[derive(Clone, Hash, Debug, PartialEq, Eq, EncodeLabelSet)]
pub struct TCPLabels {
	pub bind: DefaultedUnknown<RichStrng>,
	pub gateway: DefaultedUnknown<RichStrng>,
	pub listener: DefaultedUnknown<RichStrng>,
	pub protocol: TransportProtocol,
}

#[derive(Clone, Hash, Debug, PartialEq, Eq, EncodeLabelSet)]
pub struct ConnectLabels {
	pub transport: DefaultedUnknown<RichStrng>,
}

#[derive(Clone, Hash, Debug, PartialEq, Eq, EncodeLabelSet)]
pub struct AdmissionLabels {
	pub bind: DefaultedUnknown<RichStrng>,
}

#[derive(Clone, Hash, Debug, PartialEq, Eq, EncodeLabelSet)]
pub struct SubstrateRouteLabels {
	pub ate_router_outcome: EncodeDisplay<RouteOutcome>,
	pub ate_router_resume: EncodeDisplay<ResumeDisposition>,
}

/// One destination an actor's egress policy refused. The actor is part of the
/// series so a refusal can be shown on the Session or run that met it.
#[derive(Clone, Hash, Debug, PartialEq, Eq, EncodeLabelSet)]
pub struct EgressRefusalLabels {
	pub ate_atespace: String,
	pub ate_actor_name: String,
	/// `http`, `https` (a decrypted request or a TLS connection by its SNI) or `tcp`.
	pub scheme: &'static str,
	/// The name the actor asked for: the HTTP authority, the SNI, or the address of a tcp connection.
	pub host: String,
	pub port: u16,
}

/// How long a refused destination's series stays exported after its last
/// refusal. A series is per actor, and actors come and go, so an idle one is
/// dropped; the scraper keeps its samples.
const EGRESS_REFUSAL_IDLE: Duration = Duration::from_secs(3600);
/// How often idle series are looked for.
const EGRESS_REFUSAL_SWEEP: Duration = Duration::from_secs(60);
/// The destinations one actor has series of. The actor names the hosts, so
/// more fold into `host="other"`, `port="0"`.
const EGRESS_REFUSAL_DESTINATIONS_PER_ACTOR: usize = 16;
/// The series of all actors together. More fold into `ate_actor_name="other"`.
const EGRESS_REFUSAL_SERIES: usize = 4096;
const EGRESS_REFUSAL_OTHER: &str = "other";

/// The egress policy refusals per actor and destination, bounded by
/// [`EGRESS_REFUSAL_IDLE`], [`EGRESS_REFUSAL_DESTINATIONS_PER_ACTOR`] and
/// [`EGRESS_REFUSAL_SERIES`].
#[derive(Debug, Default, Clone)]
pub struct EgressRefusals {
	family: Family<EgressRefusalLabels, counter::Counter>,
	state: Arc<Mutex<EgressRefusalState>>,
}

#[derive(Debug, Default)]
struct EgressRefusalState {
	last_seen: HashMap<EgressRefusalLabels, Instant>,
	swept: Option<Instant>,
}

impl EgressRefusals {
	pub fn record(&self, labels: EgressRefusalLabels) {
		self.record_at(labels, Instant::now());
	}

	fn record_at(&self, mut labels: EgressRefusalLabels, now: Instant) {
		let mut state = self.state.lock();
		if state
			.swept
			.is_none_or(|swept| now.saturating_duration_since(swept) >= EGRESS_REFUSAL_SWEEP)
		{
			state.last_seen.retain(|series, seen| {
				let live = now.saturating_duration_since(*seen) < EGRESS_REFUSAL_IDLE;
				if !live {
					self.family.remove(series);
				}
				live
			});
			state.swept = Some(now);
		}
		if !state.last_seen.contains_key(&labels) {
			let actor_destinations = state
				.last_seen
				.keys()
				.filter(|series| {
					series.ate_atespace == labels.ate_atespace
						&& series.ate_actor_name == labels.ate_actor_name
						&& series.host != EGRESS_REFUSAL_OTHER
				})
				.count();
			if actor_destinations >= EGRESS_REFUSAL_DESTINATIONS_PER_ACTOR {
				labels.host = EGRESS_REFUSAL_OTHER.to_owned();
				labels.port = 0;
			}
			if !state.last_seen.contains_key(&labels) && state.last_seen.len() >= EGRESS_REFUSAL_SERIES {
				labels.ate_atespace = String::new();
				labels.ate_actor_name = EGRESS_REFUSAL_OTHER.to_owned();
				labels.host = EGRESS_REFUSAL_OTHER.to_owned();
				labels.port = 0;
			}
		}
		self.family.get_or_create(&labels).inc();
		state.last_seen.insert(labels, now);
	}

	/// The refusals one series counts, while it is exported.
	pub fn count(&self, labels: &EgressRefusalLabels) -> Option<u64> {
		self.family.get(labels).map(|counter| counter.get())
	}
}

#[derive(
	Copy, Clone, Hash, Debug, PartialEq, Eq, prometheus_client::encoding::EncodeLabelValue, Default,
)]
pub enum OutboundCallKind {
	/// The primary backend call
	#[default]
	Primary,
	/// A callout as part of a policy execution
	Policy,
	/// A mirrored call
	Mirror,
}

impl OutboundCallKind {
	pub const fn as_str(self) -> &'static str {
		match self {
			Self::Primary => "Primary",
			Self::Policy => "Policy",
			Self::Mirror => "Mirror",
		}
	}
}

#[derive(
	Copy, Clone, Hash, Debug, PartialEq, Eq, prometheus_client::encoding::EncodeLabelValue, Default,
)]
pub enum OutboundCallSubtype {
	// Primary
	#[default]
	Http,
	Llm,
	Mcp,

	// Policy
	ExtAuthz,
	Substrate,
	ExtProc,
	Guardrail,
	RateLimit,
	Oidc,
}

impl OutboundCallSubtype {
	pub const fn as_str(self) -> &'static str {
		match self {
			Self::Http => "Http",
			Self::Llm => "Llm",
			Self::Mcp => "Mcp",
			Self::ExtAuthz => "ExtAuthz",
			Self::Substrate => "Substrate",
			Self::ExtProc => "ExtProc",
			Self::Guardrail => "Guardrail",
			Self::RateLimit => "RateLimit",
			Self::Oidc => "Oidc",
		}
	}
}

#[derive(Copy, Clone, Hash, Debug, PartialEq, Eq, EncodeLabelSet)]
pub struct OutboundCallLabels {
	pub kind: OutboundCallKind,
	pub subtype: OutboundCallSubtype,
}

type Counter = Family<HTTPLabels, counter::Counter>;
type Histogram<T> = Family<T, PromHistogram, HistogramConstructor>;
type TCPCounter = Family<TCPLabels, counter::Counter>;

#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct HistogramConstructor {
	mode: HistogramMode,
	buckets: &'static [f64],
}

impl MetricConstructor<PromHistogram> for HistogramConstructor {
	fn new_metric(&self) -> PromHistogram {
		match self.mode {
			HistogramMode::Classic => PromHistogram::new(self.buckets.iter().copied()),
			HistogramMode::Native => PromHistogram::new_native(NativeHistogramConfig::default()),
			HistogramMode::Both => PromHistogram::new_classic_and_native(
				self.buckets.iter().copied(),
				NativeHistogramConfig::default(),
			),
		}
	}
}

#[derive(Clone, Hash, Debug, PartialEq, Eq, EncodeLabelSet)]
pub struct BuildLabel {
	tag: &'static str,
}

#[derive(Debug)]
pub struct Metrics {
	pub requests: Counter,
	pub request_duration: Histogram<HTTPLabels>,
	pub request_processing_duration: Histogram<MinimalHTTPLabels>,
	pub substrate_route_duration: Histogram<SubstrateRouteLabels>,
	pub response_processing_duration: Histogram<MinimalHTTPLabels>,
	pub response_bytes: Family<HTTPLabels, counter::Counter>,

	pub mcp_requests: Family<MCPCall, counter::Counter>,

	pub gen_ai_token_usage: Histogram<GenAILabelsTokenUsage>,
	pub gen_ai_cost: Family<GenAILabels, counter::Counter<f64>>,
	pub gen_ai_request_duration: Histogram<GenAIRequestDurationLabels>,
	pub gen_ai_time_per_output_token: Histogram<GenAILabels>,
	pub gen_ai_time_to_first_token: Histogram<GenAILabels>,
	pub gen_ai_inter_chunk_latency: Histogram<GenAILabels>,

	pub tls_handshake_duration: Histogram<TCPLabels>,

	pub downstream_connection: TCPCounter,
	pub tcp_downstream_rx_bytes: Family<TCPLabels, counter::Counter>,
	pub tcp_downstream_tx_bytes: Family<TCPLabels, counter::Counter>,
	pub downstream_connections_shed: Family<AdmissionLabels, counter::Counter>,
	pub requests_shed: Family<AdmissionLabels, counter::Counter>,

	pub upstream_connect_duration: Histogram<ConnectLabels>,
	pub upstream_call_duration: Histogram<OutboundCallLabels>,

	// metrics for guardrail checks (allow/mask/reject) for request/response
	pub guardrail_checks: Family<GuardrailLabels, counter::Counter>,

	pub cost_catalog_lookups: Family<CostCatalogLookupLabels, counter::Counter>,

	// metrics for request retries
	pub retries: Counter,

	// Number of requests currently waiting for a Substrate actor to become routable.
	pub substrate_request_parking_active: Gauge,

	// Destinations an actor's egress policy refused, per actor.
	pub substrate_egress_refusals: EgressRefusals,
}

// FilteredRegistry is a wrapper around Registry that allows to filter out certain metrics.
// Note: this currently only excludes them from the registry, but the underlying metrics are still
// stored. This can result in memory cost, etc to store the labels.
// A more robust future solution would be to have a sort of `Disabled` metric that does not store;
// note that even still, we would be computing the labels (and then dropping them), but in many cases
// the same labels are shared by many metrics, and are cheap to construct, so likely not a major concern.
struct FilteredRegistry<'a, R> {
	registry: &'a mut R,
	removes: FzHashSet<String>,
}

impl<R: MetricRegistry> FilteredRegistry<'_, R> {
	fn should_skip(&self, name: &str, unit: Option<&Unit>) -> bool {
		let mut names = vec![
			name.to_string(),
			format!("{}_total", name),
			format!("{}_{}_total", agent_core::metrics::PREFIX, name),
			format!("{}_{}", agent_core::metrics::PREFIX, name),
		];
		if let Some(unit) = unit {
			names.extend_from_slice(&[
				format!("{}_{}", name, unit.as_str()),
				format!("{}_{}_total", name, unit.as_str()),
				format!(
					"{}_{}_{}_total",
					agent_core::metrics::PREFIX,
					name,
					unit.as_str()
				),
				format!("{}_{}_{}", agent_core::metrics::PREFIX, name, unit.as_str()),
			])
		}

		for n in names.into_iter() {
			let exclude = self.removes.contains(&n);
			trace!(name = n, exclude, "check metric for exclusion");
			if exclude {
				return true;
			}
		}
		false
	}
	fn register(&mut self, name: impl Into<String>, help: impl Into<String>, metric: impl Metric) {
		let name = name.into();
		if self.should_skip(&name, None) {
			debug!("skip register metric: {}", name);
			return;
		}
		self.registry.register(name, help, metric);
	}

	fn register_with_unit(
		&mut self,
		name: impl Into<String>,
		help: impl Into<String>,
		unit: Unit,
		metric: impl Metric,
	) {
		let name = name.into();
		if self.should_skip(&name, Some(&unit)) {
			debug!("skip register metric: {}_{}", name, unit.as_str());
			return;
		}
		self.registry.register_with_unit(name, help, unit, metric);
	}
}

impl Metrics {
	pub fn new<R: MetricRegistry>(
		registry: &mut R,
		removes: FzHashSet<String>,
		histogram_mode: HistogramMode,
	) -> Self {
		let mut registry = FilteredRegistry { registry, removes };
		registry.register(
			"build",
			"Agentgateway build information",
			Info::new(BuildLabel {
				tag: version::BuildInfo::new().version,
			}),
		);

		let gen_ai_token_usage = histogram_family(histogram_mode, &TOKEN_USAGE_BUCKET);
		registry.register(
			"gen_ai_client_token_usage",
			"Number of tokens used per request",
			gen_ai_token_usage.clone(),
		);

		let gen_ai_cost = Family::<GenAILabels, _>::default();
		registry.register_with_unit(
			"gen_ai_client_cost",
			"Cumulative USD cost of generative AI requests",
			Unit::Other("usd".to_string()),
			gen_ai_cost.clone(),
		);

		let gen_ai_request_duration = histogram_family(histogram_mode, &REQUEST_DURATION_BUCKET);
		registry.register(
			"gen_ai_server_request_duration",
			"Duration of a generative AI request in seconds; failed operations have error_type=\"_OTHER\" and successful operations omit the label",
			gen_ai_request_duration.clone(),
		);

		let gen_ai_time_per_output_token = histogram_family(histogram_mode, &OUTPUT_TOKEN_BUCKET);
		registry.register(
			"gen_ai_server_time_per_output_token",
			"Time to generate each output token for a given request",
			gen_ai_time_per_output_token.clone(),
		);

		let gen_ai_time_to_first_token = histogram_family(histogram_mode, &FIRST_TOKEN_BUCKET);
		registry.register(
			"gen_ai_server_time_to_first_token",
			"Time to generate the first token for a given request",
			gen_ai_time_to_first_token.clone(),
		);

		let gen_ai_inter_chunk_latency = histogram_family(histogram_mode, &OUTPUT_TOKEN_BUCKET);
		registry.register(
			"gen_ai_server_inter_chunk_latency",
			"Time between consecutive output chunks for a given request",
			gen_ai_inter_chunk_latency.clone(),
		);

		Metrics {
			substrate_request_parking_active: {
				let m = Gauge::default();
				registry.register(
					"substrate_request_parking_active",
					"Number of requests waiting for a Substrate actor to become routable",
					m.clone(),
				);
				m
			},
			substrate_egress_refusals: {
				let m = EgressRefusals::default();
				registry.register(
					"atenet_egress_refusals",
					"Requests and connections an actor's egress policy refused, per actor and destination",
					m.family.clone(),
				);
				m
			},
			requests: build(
				&mut registry,
				"requests",
				"The total number of HTTP requests sent",
			),
			guardrail_checks: {
				let m = Family::<GuardrailLabels, _>::default();
				registry.register(
					"guardrail_checks",
					"Total number of guardrail checks",
					m.clone(),
				);
				m
			},
			cost_catalog_lookups: {
				let m = Family::<CostCatalogLookupLabels, _>::default();
				registry.register(
					"cost_catalog_lookups",
					"Total number of model cost catalog lookups by resolution status",
					m.clone(),
				);
				m
			},
			downstream_connection: build(
				&mut registry,
				"downstream_connections",
				"The total number of downstream connections established",
			),
			downstream_connections_shed: build(
				&mut registry,
				"downstream_connections_shed",
				"Total downstream connections closed by the active connection limit",
			),
			requests_shed: build(
				&mut registry,
				"requests_shed",
				"Total downstream requests rejected by the in-flight request limit",
			),

			mcp_requests: build(
				&mut registry,
				"mcp_requests",
				"Total number of MCP requests",
			),

			gen_ai_token_usage,
			gen_ai_cost,
			gen_ai_request_duration,
			gen_ai_time_per_output_token,
			gen_ai_time_to_first_token,
			gen_ai_inter_chunk_latency,

			response_bytes: {
				let m = Family::<HTTPLabels, _>::default();
				registry.register_with_unit(
					"response",
					"Total HTTP response bytes received",
					Unit::Bytes,
					m.clone(),
				);
				m
			},
			request_duration: {
				let m = histogram_family(histogram_mode, &HTTP_REQUEST_DURATION_BUCKET);
				registry.register_with_unit(
					"request_duration",
					"Duration of HTTP requests (seconds)",
					Unit::Seconds,
					m.clone(),
				);
				m
			},
			request_processing_duration: {
				let m = histogram_family(histogram_mode, &PROCESSING_DURATION_BUCKETS);
				registry.register_with_unit(
					"request_processing",
					"Duration from receiving an HTTP request to sending the primary outbound call (seconds)",
					Unit::Seconds,
					m.clone(),
				);
				m
			},
			substrate_route_duration: {
				let m = histogram_family(histogram_mode, &HTTP_REQUEST_DURATION_BUCKET);
				registry.register_with_unit(
					"atenet_router_route_duration",
					"Time from receiving a Substrate request to resolving its worker endpoint",
					Unit::Seconds,
					m.clone(),
				);
				m
			},
			response_processing_duration: {
				let m = histogram_family(histogram_mode, &PROCESSING_DURATION_BUCKETS);
				registry.register_with_unit(
					"response_processing",
					"Duration from receiving the primary outbound response to sending the HTTP response (seconds)",
					Unit::Seconds,
					m.clone(),
				);
				m
			},
			tcp_downstream_rx_bytes: {
				let m = Family::<TCPLabels, _>::default();
				registry.register_with_unit(
					"downstream_received",
					"Total TCP bytes received per connection labels",
					Unit::Bytes,
					m.clone(),
				);
				m
			},
			tcp_downstream_tx_bytes: {
				let m = Family::<TCPLabels, _>::default();
				registry.register_with_unit(
					"downstream_sent",
					"Total TCP bytes transmitted per connection labels",
					Unit::Bytes,
					m.clone(),
				);
				m
			},
			upstream_connect_duration: {
				let m = histogram_family(histogram_mode, &CONNECT_DURATION_BUCKET);
				registry.register_with_unit(
					"upstream_connect_duration",
					"Duration to establish upstream connection (seconds)",
					Unit::Seconds,
					m.clone(),
				);
				m
			},
			upstream_call_duration: {
				let m = histogram_family(histogram_mode, &HTTP_REQUEST_DURATION_BUCKET);
				registry.register_with_unit(
					"upstream_call_duration",
					"Duration of outbound calls made by agentgateway (seconds)",
					Unit::Seconds,
					m.clone(),
				);
				m
			},
			tls_handshake_duration: {
				let m = histogram_family(histogram_mode, &CONNECT_DURATION_BUCKET);
				registry.register_with_unit(
					"tls_handshake_duration",
					"Duration to complete inbound TLS/HTTPS handshake (seconds)",
					Unit::Seconds,
					m.clone(),
				);
				m
			},
			retries: build(
				&mut registry,
				"retries",
				"The total number of request retries",
			),
		}
	}
}

fn histogram_family<T>(mode: HistogramMode, buckets: &'static [f64]) -> Histogram<T>
where
	T: Clone + std::hash::Hash + Eq,
{
	Family::new_with_constructor(HistogramConstructor { mode, buckets })
}

fn build<T: Clone + std::hash::Hash + Eq + Send + Sync + Debug + EncodeLabelSet + 'static>(
	registry: &mut FilteredRegistry<'_, impl MetricRegistry>,
	name: &str,
	help: &str,
) -> Family<T, counter::Counter> {
	let m = Family::<T, _>::default();
	registry.register(name, help, m.clone());
	m
}

// https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-metrics/#metric-gen_aiclienttokenusage
const TOKEN_USAGE_BUCKET: [f64; 14] = [
	1., 4., 16., 64., 256., 1024., 4096., 16384., 65536., 262144., 1048576., 4194304., 16777216.,
	67108864.,
];
// https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-metrics/#metric-gen_aiserverrequestduration
const REQUEST_DURATION_BUCKET: [f64; 14] = [
	0.01, 0.02, 0.04, 0.08, 0.16, 0.32, 0.64, 1.28, 2.56, 5.12, 10.24, 20.48, 40.96, 81.92,
];
// Finer-grained, exponentially growing buckets for TCP/TLS connect.
// Keep in seconds (Prometheus convention). Prioritize sub-second resolution, with a few larger outlier buckets.
const CONNECT_DURATION_BUCKET: [f64; 10] = [
	0.0005, // 0.5 ms
	0.0015, // 1.5 ms
	0.0043, // 4.3 ms
	0.0126, // 12.6 ms
	0.0368, // 36.8 ms
	0.108,  // 108 ms
	0.316,  // 316 ms
	0.924,  // 924 ms
	2.71,   // 2.71 s
	8.0,    // 8 s
];
// HTTP request duration buckets - general purpose for all HTTP traffic
// Covers 1ms to ~80 seconds with exponential growth
const HTTP_REQUEST_DURATION_BUCKET: [f64; 14] = [
	0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 80.0,
];
// Internal processing time
// Covers 50us to 250ms with growth.
const PROCESSING_DURATION_BUCKETS: [f64; 10] = [
	0.00005, // 50us
	0.0001,  // 100us
	0.00025, // 250us
	0.0005,  // 500us
	0.001,   // 1ms
	0.0025,  // 2.5ms
	0.005,   // 5ms
	0.01,    // 10ms
	0.05,    // 50ms
	0.25,    // 250ms
];

// https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-metrics/#metric-gen_aiservertime_per_output_token
// NOTE: the spec has SHOULD, but is not smart enough to handle the faster LLMs.
// We have added 0.001 (1000 TPS)
const OUTPUT_TOKEN_BUCKET: [f64; 14] = [
	0.001, 0.01, 0.025, 0.05, 0.075, 0.1, 0.15, 0.2, 0.3, 0.4, 0.5, 0.75, 1.0, 2.5,
];
// https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-metrics/#metric-gen_aiservertime_to_first_token
const FIRST_TOKEN_BUCKET: [f64; 16] = [
	0.001, 0.005, 0.01, 0.02, 0.04, 0.06, 0.08, 0.1, 0.25, 0.5, 0.75, 1.0, 2.5, 5.0, 7.5, 10.0,
];

#[cfg(test)]
mod tests {
	use prometheus_client::encoding::prometheus_protobuf;
	use prometheus_client::registry::Registry;

	use super::*;

	fn refusal(actor: &str) -> EgressRefusalLabels {
		EgressRefusalLabels {
			ate_atespace: "agents".to_owned(),
			ate_actor_name: actor.to_owned(),
			scheme: "https",
			host: "proxy.golang.org".to_owned(),
			port: 443,
		}
	}

	#[test]
	fn an_egress_refusal_series_is_dropped_once_idle() {
		let refusals = EgressRefusals::default();
		let start = Instant::now();
		refusals.record_at(refusal("session-a"), start);
		refusals.record_at(refusal("session-a"), start);
		refusals.record_at(refusal("session-b"), start + EGRESS_REFUSAL_IDLE / 2);
		assert_eq!(refusals.count(&refusal("session-a")), Some(2));

		refusals.record_at(refusal("session-b"), start + EGRESS_REFUSAL_IDLE);
		assert_eq!(refusals.count(&refusal("session-a")), None);
		assert_eq!(refusals.count(&refusal("session-b")), Some(2));
	}

	#[test]
	fn an_actors_destinations_beyond_the_bound_fold_into_other() {
		let refusals = EgressRefusals::default();
		let now = Instant::now();
		let destination = |host: String| EgressRefusalLabels {
			host,
			..refusal("session-a")
		};
		for n in 0..EGRESS_REFUSAL_DESTINATIONS_PER_ACTOR + 2 {
			refusals.record_at(destination(format!("host-{n}.example")), now);
		}
		refusals.record_at(destination("host-0.example".to_owned()), now);
		refusals.record_at(refusal("session-b"), now);

		assert_eq!(
			refusals.count(&destination("host-0.example".to_owned())),
			Some(2)
		);
		assert_eq!(
			refusals.count(&destination(format!(
				"host-{EGRESS_REFUSAL_DESTINATIONS_PER_ACTOR}.example"
			))),
			None
		);
		assert_eq!(
			refusals.count(&EgressRefusalLabels {
				port: 0,
				..destination(EGRESS_REFUSAL_OTHER.to_owned())
			}),
			Some(2)
		);
		assert_eq!(refusals.count(&refusal("session-b")), Some(1));
	}

	#[test]
	fn histogram_mode_controls_collected_representations() {
		for (mode, want_classic, want_native) in [
			(HistogramMode::Classic, true, false),
			(HistogramMode::Native, false, true),
			(HistogramMode::Both, true, true),
		] {
			let mut registry = Registry::default();
			let metrics = Metrics::new(&mut registry, Default::default(), mode);
			metrics
				.request_duration
				.get_or_create(&HTTPLabels::default())
				.observe(1.0);

			let families = prometheus_protobuf::encode(&registry).expect("protobuf encoding succeeds");
			let histogram = families
				.iter()
				.find(|family| family.name == "request_duration_seconds")
				.and_then(|family| family.metric.first())
				.and_then(|metric| metric.histogram.as_ref())
				.expect("request duration histogram is encoded");

			assert_eq!(!histogram.bucket.is_empty(), want_classic, "mode: {mode:?}");
			assert_eq!(
				!histogram.positive_span.is_empty(),
				want_native,
				"mode: {mode:?}"
			);
		}
	}
}

//! The data plane's side of the control plane's `JwksRefresh` service.
//!
//! Under Kubernetes the control plane fetches a provider's remote JWKS and pushes the keys
//! inline over xDS, on a schedule. Between two fetches the issuer may rotate its signing key,
//! and every token it signs with the new key names a key id the pushed set does not have.
//! A provider whose keys came that way carries the control plane's key for the fetch, and
//! [`ControlPlaneJwks`] asks the control plane for a refetch over the channel the data plane
//! holds to it already, once per key and interval; the requests that meet the key id meanwhile
//! await that call's answer.
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use futures::future::{BoxFuture, FutureExt, Shared};
use tracing::{debug, warn};

use crate::control::GrpcChannel;
use crate::http::jwt::JwksRefreshSource;
use crate::types::proto::agent::JwksRefreshRequest;
use crate::types::proto::agent::jwks_refresh_client::JwksRefreshClient;

/// At most one call per key within this interval: a burst of tokens with unknown key ids
/// costs the control plane one call, whatever the number of tokens.
const MIN_INTERVAL: Duration = Duration::from_secs(10);
/// The requests that meet the unknown key id wait at most this long for the answer. The
/// control plane's own fetch carries on, and a changed set reaches the data plane through the
/// policy update.
const TIMEOUT: Duration = Duration::from_secs(3);

/// Asks the control plane to refetch a remote JWKS it pushed inline, over the channel the data
/// plane holds to it already (the xDS address, authentication and CA).
#[derive(Debug)]
pub struct ControlPlaneJwks {
	channel: GrpcChannel,
	calls: RefreshCalls,
}

impl ControlPlaneJwks {
	pub fn new(channel: GrpcChannel) -> Self {
		Self {
			channel,
			calls: RefreshCalls::default(),
		}
	}
}

impl JwksRefreshSource for ControlPlaneJwks {
	fn refresh<'a>(
		&'a self,
		key: &'a str,
		kid: &'a str,
	) -> Pin<Box<dyn Future<Output = Option<String>> + Send + 'a>> {
		let channel = self.channel.clone();
		let call = self.calls.get_or_start(key, || {
			let request = JwksRefreshRequest {
				remote_jwks_key: key.to_owned(),
				kid: kid.to_owned(),
			};
			ask(channel, request).boxed()
		});
		Box::pin(call)
	}
}

/// One call to the control plane, bounded by [`TIMEOUT`].
async fn ask(channel: GrpcChannel, request: JwksRefreshRequest) -> Option<String> {
	let key = request.remote_jwks_key.clone();
	let kid = request.kid.clone();
	let mut client = JwksRefreshClient::new(channel);
	match tokio::time::timeout(TIMEOUT, client.refresh(request)).await {
		Ok(Ok(response)) => {
			let response = response.into_inner();
			debug!(
				key,
				kid,
				refetched = response.refetched,
				"control plane answered the JWKS refresh"
			);
			Some(response.jwks)
		},
		Ok(Err(status)) => {
			warn!(key, kid, %status, "control plane refused the JWKS refresh");
			None
		},
		Err(_) => {
			warn!(key, kid, timeout = ?TIMEOUT, "control plane did not answer the JWKS refresh in time");
			None
		},
	}
}

type RefreshCall = Shared<BoxFuture<'static, Option<String>>>;

/// Each key's current call and when it started. Every request that meets an unknown key id
/// within the key's interval awaits the same call: one still in flight is awaited, not
/// duplicated, and one that completed hands out its answer at once, so the tokens that meet
/// the rotated key until the control plane's policy update arrives validate against it. A
/// failed call answers `None` for the rest of its interval.
#[derive(Debug, Default)]
pub(crate) struct RefreshCalls {
	calls: Mutex<HashMap<String, (Instant, RefreshCall)>>,
}

impl RefreshCalls {
	/// The call for `key` in its current interval, started with `start` when there is none.
	pub(crate) fn get_or_start(
		&self,
		key: &str,
		start: impl FnOnce() -> BoxFuture<'static, Option<String>>,
	) -> RefreshCall {
		let mut calls = self.calls.lock().expect("JWKS refresh calls poisoned");
		let now = Instant::now();
		if let Some((started, call)) = calls.get(key)
			&& now.duration_since(*started) < MIN_INTERVAL
		{
			debug!(key, "JWKS refresh joins the interval's call");
			return call.clone();
		}
		let call = start().shared();
		calls.insert(key.to_owned(), (now, call.clone()));
		call
	}
}

#[cfg(test)]
mod tests {
	use std::sync::Arc;
	use std::sync::atomic::{AtomicUsize, Ordering};

	use super::*;

	/// A call that counts its starts and answers `answer` once `gate` opens.
	fn gated(
		starts: &Arc<AtomicUsize>,
		gate: &Arc<tokio::sync::Notify>,
		answer: &str,
	) -> impl FnOnce() -> BoxFuture<'static, Option<String>> {
		let starts = starts.clone();
		let gate = gate.clone();
		let answer = answer.to_owned();
		move || {
			starts.fetch_add(1, Ordering::SeqCst);
			async move {
				gate.notified().await;
				Some(answer)
			}
			.boxed()
		}
	}

	#[tokio::test]
	async fn a_call_in_flight_is_awaited_not_duplicated() {
		let calls = RefreshCalls::default();
		let starts = Arc::new(AtomicUsize::new(0));
		let gate = Arc::new(tokio::sync::Notify::new());

		let first = calls.get_or_start("a", gated(&starts, &gate, "jwks-1"));
		let second = calls.get_or_start("a", gated(&starts, &gate, "jwks-2"));
		assert_eq!(starts.load(Ordering::SeqCst), 1, "one call for both");

		let both = tokio::spawn(async move { futures::join!(first, second) });
		tokio::task::yield_now().await;
		assert!(!both.is_finished(), "both wait for the call in flight");
		gate.notify_one();
		assert_eq!(
			both.await.unwrap(),
			(Some("jwks-1".to_owned()), Some("jwks-1".to_owned())),
			"both get the call's answer"
		);
	}

	#[tokio::test]
	async fn one_call_per_key_and_interval() {
		let calls = RefreshCalls::default();
		let starts = Arc::new(AtomicUsize::new(0));
		let gate = Arc::new(tokio::sync::Notify::new());
		gate.notify_one();

		assert_eq!(
			calls
				.get_or_start("a", gated(&starts, &gate, "jwks-1"))
				.await,
			Some("jwks-1".to_owned())
		);
		assert_eq!(
			calls
				.get_or_start("a", gated(&starts, &gate, "jwks-2"))
				.await,
			Some("jwks-1".to_owned()),
			"a later request within the interval gets the interval's answer at once"
		);
		assert_eq!(starts.load(Ordering::SeqCst), 1);

		gate.notify_one();
		assert_eq!(
			calls
				.get_or_start("b", gated(&starts, &gate, "jwks-b"))
				.await,
			Some("jwks-b".to_owned()),
			"another key has its own call"
		);
		assert_eq!(starts.load(Ordering::SeqCst), 2);

		let answered = calls.calls.lock().unwrap().get("a").unwrap().1.clone();
		calls
			.calls
			.lock()
			.unwrap()
			.insert("a".to_owned(), (Instant::now() - MIN_INTERVAL, answered));
		gate.notify_one();
		assert_eq!(
			calls
				.get_or_start("a", gated(&starts, &gate, "jwks-3"))
				.await,
			Some("jwks-3".to_owned()),
			"the interval over, the key is asked again"
		);
		assert_eq!(starts.load(Ordering::SeqCst), 3);
	}
}

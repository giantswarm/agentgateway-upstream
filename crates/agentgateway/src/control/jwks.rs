//! The data plane's side of the control plane's `JwksRefresh` service.
//!
//! Under Kubernetes the control plane fetches a provider's remote JWKS and pushes the keys
//! inline over xDS, on a schedule. Between two fetches the issuer may rotate its signing key,
//! and every token it signs with the new key names a key id the pushed set does not have.
//! A provider whose keys came that way carries the control plane's key for the fetch, and
//! [`ControlPlaneJwks`] asks the control plane for a refetch over the channel the data plane
//! holds to it already, once per key and interval.
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tracing::{debug, warn};

use crate::control::GrpcChannel;
use crate::http::jwt::JwksRefreshSource;
use crate::types::proto::agent::JwksRefreshRequest;
use crate::types::proto::agent::jwks_refresh_client::JwksRefreshClient;

/// At most one call per key within this interval: a burst of tokens with unknown key ids
/// costs the control plane one call, whatever the number of tokens.
const MIN_INTERVAL: Duration = Duration::from_secs(10);
/// The request that met the unknown key id waits at most this long for the answer. The
/// control plane's own fetch carries on, and a changed set reaches the data plane through the
/// policy update.
const TIMEOUT: Duration = Duration::from_secs(3);

/// Asks the control plane to refetch a remote JWKS it pushed inline, over the channel the data
/// plane holds to it already (the xDS address, authentication and CA).
#[derive(Debug)]
pub struct ControlPlaneJwks {
	channel: GrpcChannel,
	intervals: RefreshIntervals,
}

impl ControlPlaneJwks {
	pub fn new(channel: GrpcChannel) -> Self {
		Self {
			channel,
			intervals: RefreshIntervals::default(),
		}
	}
}

impl JwksRefreshSource for ControlPlaneJwks {
	fn refresh<'a>(
		&'a self,
		key: &'a str,
		kid: &'a str,
	) -> Pin<Box<dyn Future<Output = Option<String>> + Send + 'a>> {
		Box::pin(async move {
			if let Some(last) = self.intervals.start(key) {
				debug!(key, kid, "JWKS refresh skipped: asked within the interval");
				return last;
			}
			let mut client = JwksRefreshClient::new(self.channel.clone());
			let request = JwksRefreshRequest {
				remote_jwks_key: key.to_owned(),
				kid: kid.to_owned(),
			};
			let jwks = match tokio::time::timeout(TIMEOUT, client.refresh(request)).await {
				Ok(Ok(response)) => {
					let response = response.into_inner();
					debug!(
						key,
						kid,
						refetched = response.refetched,
						"control plane answered the JWKS refresh"
					);
					response.jwks
				},
				Ok(Err(status)) => {
					warn!(key, kid, %status, "control plane refused the JWKS refresh");
					return None;
				},
				Err(_) => {
					warn!(key, kid, timeout = ?TIMEOUT, "control plane did not answer the JWKS refresh in time");
					return None;
				},
			};
			self.intervals.answered(key, &jwks);
			Some(jwks)
		})
	}
}

/// Each key's current interval: its start and the document the control plane answered in it.
#[derive(Debug, Default)]
struct RefreshIntervals {
	last: Mutex<HashMap<String, (Instant, Option<String>)>>,
}

impl RefreshIntervals {
	/// `None` when a call for `key` may start now, which starts the key's interval before the
	/// call runs, so a failing call consumes the interval as well. Within the interval,
	/// `Some` with the document answered in it, if one was: the tokens that meet the rotated
	/// key until the control plane's policy update arrives validate against it.
	fn start(&self, key: &str) -> Option<Option<String>> {
		let mut last = self.last.lock().expect("JWKS refresh intervals poisoned");
		let now = Instant::now();
		if let Some((started, jwks)) = last.get(key)
			&& now.duration_since(*started) < MIN_INTERVAL
		{
			return Some(jwks.clone());
		}
		last.insert(key.to_owned(), (now, None));
		None
	}

	/// Keeps the document the control plane answered for `key` for the rest of its interval.
	fn answered(&self, key: &str, jwks: &str) {
		let mut last = self.last.lock().expect("JWKS refresh intervals poisoned");
		if let Some((_, answer)) = last.get_mut(key) {
			*answer = Some(jwks.to_owned());
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn one_call_per_key_and_interval() {
		let intervals = RefreshIntervals::default();
		assert_eq!(intervals.start("a"), None, "the first call starts");
		assert_eq!(
			intervals.start("a"),
			Some(None),
			"a second call within the interval is skipped, nothing answered yet"
		);
		intervals.answered("a", "{\"keys\":[]}");
		assert_eq!(
			intervals.start("a"),
			Some(Some("{\"keys\":[]}".to_owned())),
			"a skipped call gets the interval's answer"
		);
		assert_eq!(
			intervals.start("b"),
			None,
			"another key has its own interval"
		);
		intervals
			.last
			.lock()
			.unwrap()
			.insert("a".to_owned(), (Instant::now() - MIN_INTERVAL, None));
		assert_eq!(
			intervals.start("a"),
			None,
			"the interval over, the key may be asked again"
		);
	}
}

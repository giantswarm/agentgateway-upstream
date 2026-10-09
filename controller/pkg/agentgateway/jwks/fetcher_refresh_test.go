package jwks

import (
	"context"
	"encoding/json"
	"errors"
	"sync/atomic"
	"testing"
	"time"

	"github.com/go-jose/go-jose/v4"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"

	"github.com/agentgateway/agentgateway/controller/pkg/agentgateway/remotehttp"
)

// countingJwksClient counts the fetches it forwards to inner.
type countingJwksClient struct {
	inner JwksHttpClient
	calls *atomic.Int32
}

func (c countingJwksClient) FetchJwks(ctx context.Context, target remotehttp.FetchTarget) (jose.JSONWebKeySet, error) {
	c.calls.Add(1)
	return c.inner.FetchJwks(ctx, target)
}

func sampleKeyset(t *testing.T) jose.JSONWebKeySet {
	t.Helper()
	keys := jose.JSONWebKeySet{}
	require.NoError(t, json.Unmarshal([]byte(sampleJWKS), &keys))
	return keys
}

func TestRefreshNowFetchesCommitsAndNotifiesWhenRequestIsLive(t *testing.T) {
	ctx := t.Context()
	source := testSource()
	fetches := &atomic.Int32{}
	f := NewFetcher(NewCache())
	f.defaultJwksClient = countingJwksClient{
		inner: stubJwksClient{t: t, expectedReq: source.Target, result: sampleKeyset(t)},
		calls: fetches,
	}
	require.NoError(t, f.AddOrUpdateKeyset(source))
	updates := f.SubscribeToUpdates()

	keyset, refetched, err := f.RefreshNow(ctx, source, "rotated")
	require.NoError(t, err)
	assert.True(t, refetched)
	assert.Equal(t, int32(1), fetches.Load())
	assert.Equal(t, source.Target.URL, keyset.URL)
	assert.Contains(t, keyset.JwksJSON, `"kid":"JWxVLtipR-Q6wF2zmQKEoxbFhqwibK2aKNLyRqNxdj4"`)

	// The live request's cache is updated, its subscribers hear of it and its
	// next scheduled fetch moves out by the TTL.
	cached, ok := f.cache.GetJwks(source.RequestKey)
	assert.True(t, ok)
	assert.Equal(t, keyset.JwksJSON, cached.JwksJSON)
	awaitJwksUpdate(t, updates, source.RequestKey)
	f.mu.Lock()
	next := f.schedule.Peek()
	f.mu.Unlock()
	require.NotNil(t, next)
	assert.WithinDuration(t, time.Now().Add(source.TTL), next.At, time.Minute)

	// Within the interval a second call answers the cache without a fetch.
	again, refetched, err := f.RefreshNow(ctx, source, "rotated")
	require.NoError(t, err)
	assert.False(t, refetched)
	assert.Equal(t, keyset.JwksJSON, again.JwksJSON)
	assert.Equal(t, int32(1), fetches.Load())
}

func TestRefreshNowWithoutLiveRequestAnswersWithoutCommitting(t *testing.T) {
	// A replica that does not run the schedule holds no request state: it
	// answers the data plane and persists nothing.
	source := testSource()
	f := NewFetcher(NewCache())
	f.defaultJwksClient = stubJwksClient{t: t, expectedReq: source.Target, result: sampleKeyset(t)}
	updates := f.SubscribeToUpdates()

	keyset, refetched, err := f.RefreshNow(t.Context(), source, "rotated")
	require.NoError(t, err)
	assert.True(t, refetched)
	assert.NotEmpty(t, keyset.JwksJSON)
	_, cached := f.cache.GetJwks(source.RequestKey)
	assert.False(t, cached, "nothing is committed without a live request")
	select {
	case <-updates:
		t.Fatal("no subscriber update without a live request")
	default:
	}
}

func TestRefreshNowAnswersAHeldKeysetWithTheKidWithoutFetching(t *testing.T) {
	// A replica that does not run the schedule answers every data plane asking
	// for the rotated kid from the keyset its fetch answered, past the interval,
	// until the replica that does pushes it.
	source := testSource()
	fetches := &atomic.Int32{}
	f := NewFetcher(NewCache())
	f.defaultJwksClient = countingJwksClient{
		inner: stubJwksClient{t: t, expectedReq: source.Target, result: sampleKeyset(t)},
		calls: fetches,
	}
	const kid = "JWxVLtipR-Q6wF2zmQKEoxbFhqwibK2aKNLyRqNxdj4"

	_, refetched, err := f.RefreshNow(t.Context(), source, kid)
	require.NoError(t, err)
	assert.True(t, refetched)

	f.mu.Lock()
	state := f.onDemand[source.RequestKey]
	state.started = time.Now().Add(-onDemandRefreshMinInterval)
	f.onDemand[source.RequestKey] = state
	f.mu.Unlock()

	held, refetched, err := f.RefreshNow(t.Context(), source, kid)
	require.NoError(t, err)
	assert.False(t, refetched, "the held keyset has the kid")
	assert.Contains(t, held.JwksJSON, kid)
	assert.Equal(t, int32(1), fetches.Load())

	// A kid the held keyset lacks is fetched again once the interval is over.
	_, refetched, err = f.RefreshNow(t.Context(), source, "rotated-again")
	require.NoError(t, err)
	assert.True(t, refetched)
	assert.Equal(t, int32(2), fetches.Load())
}

func TestRefreshNowFailedFetchConsumesTheInterval(t *testing.T) {
	source := testSource()
	f := NewFetcher(NewCache())
	f.defaultJwksClient = stubJwksClient{t: t, expectedReq: source.Target, err: errors.New("issuer down")}

	_, _, err := f.RefreshNow(t.Context(), source, "rotated")
	assert.ErrorContains(t, err, "issuer down")

	_, _, err = f.RefreshNow(t.Context(), source, "rotated")
	assert.ErrorIs(t, err, errRefreshSkipped)
}

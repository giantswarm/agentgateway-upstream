package jwks

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
	"istio.io/istio/pkg/kube/krt"
	"istio.io/istio/pkg/test"
	corev1 "k8s.io/api/core/v1"

	"github.com/agentgateway/agentgateway/api"
)

func TestRefreshServiceAnswersAKnownKeyAndRefusesTheRest(t *testing.T) {
	stop := test.NewStop(t)
	source := testSource()
	requests := krt.NewStaticCollection(alwaysSynced{}, []SharedJwksRequest{{
		RequestKey: source.RequestKey,
		Target:     source.Target,
		TTL:        source.TTL,
	}}, krt.WithName("jwks/RefreshServiceRequests"), krt.WithStop(stop))
	persisted := NewPersistedEntriesFromCollection(
		krt.NewStaticCollection[*corev1.ConfigMap](alwaysSynced{}, nil, krt.WithName("jwks/RefreshServiceConfigMaps"), krt.WithStop(stop)),
		DefaultJwksStorePrefix,
		"agentgateway-system",
		krt.WithStop(stop),
	)
	store := NewStore(requests, persisted, DefaultJwksStorePrefix)
	store.jwksFetcher.defaultJwksClient = stubJwksClient{t: t, expectedReq: source.Target, result: sampleKeyset(t)}
	service := NewRefreshService(requests, store)

	resp, err := service.Refresh(t.Context(), &api.JwksRefreshRequest{RemoteJwksKey: string(source.RequestKey), Kid: "rotated"})
	require.NoError(t, err)
	assert.True(t, resp.GetRefetched())
	assert.Contains(t, resp.GetJwks(), `"kid":"JWxVLtipR-Q6wF2zmQKEoxbFhqwibK2aKNLyRqNxdj4"`)

	// Within the interval the answer is the held keyset, not a fetch.
	resp, err = service.Refresh(t.Context(), &api.JwksRefreshRequest{RemoteJwksKey: string(source.RequestKey), Kid: "rotated"})
	require.NoError(t, err)
	assert.False(t, resp.GetRefetched())
	assert.NotEmpty(t, resp.GetJwks())

	_, err = service.Refresh(t.Context(), &api.JwksRefreshRequest{RemoteJwksKey: "not-a-request", Kid: "rotated"})
	assert.Equal(t, codes.NotFound, status.Code(err))

	_, err = service.Refresh(t.Context(), &api.JwksRefreshRequest{Kid: "rotated"})
	assert.Equal(t, codes.InvalidArgument, status.Code(err))
}

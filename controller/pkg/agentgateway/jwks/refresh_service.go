package jwks

import (
	"context"
	"errors"

	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
	"istio.io/istio/pkg/kube/krt"

	"github.com/agentgateway/agentgateway/api"
	"github.com/agentgateway/agentgateway/controller/pkg/agentgateway/remotehttp"
)

// RefreshService answers a data plane's JwksRefresh call. The data plane calls
// when a token names a key id the inline JWKS pushed for a provider does not
// have, which is what a signing-key rotation of the issuer looks like between
// two scheduled fetches. The shared request is found by the remote JWKS key
// the policy translation put next to the inline keys, fetched at once through
// the store (one fetch per key and interval, whatever the number of data planes
// asking), and the keyset the store holds afterwards is answered, so the
// request that asked validates without waiting for the policy update that
// follows a changed set.
type RefreshService struct {
	api.UnimplementedJwksRefreshServer
	requests krt.Collection[SharedJwksRequest]
	store    *Store
}

func NewRefreshService(requests krt.Collection[SharedJwksRequest], store *Store) *RefreshService {
	return &RefreshService{requests: requests, store: store}
}

func (s *RefreshService) Refresh(ctx context.Context, req *api.JwksRefreshRequest) (*api.JwksRefreshResponse, error) {
	requestKey := remotehttp.FetchKey(req.GetRemoteJwksKey())
	if requestKey == "" {
		return nil, status.Error(codes.InvalidArgument, "remote_jwks_key is required")
	}
	request := s.requests.GetKey(string(requestKey))
	if request == nil {
		return nil, status.Errorf(codes.NotFound, "no remote JWKS is fetched under key %q", requestKey)
	}

	keyset, refetched, err := s.store.RefreshNow(ctx, request.JwksSource())
	if err != nil {
		if !errors.Is(err, errRefreshSkipped) {
			logger.Error("error fetching jwks for a data plane's refresh request", "request_key", requestKey, "kid", req.GetKid(), "url", request.Target.URL, "error", err)
		}
		return nil, status.Errorf(codes.Unavailable, "refreshing the JWKS for key %q: %v", requestKey, err)
	}
	logger.Info("answered a data plane's jwks refresh request", "request_key", requestKey, "kid", req.GetKid(), "url", request.Target.URL, "refetched", refetched)
	return &api.JwksRefreshResponse{Jwks: keyset.JwksJSON, Refetched: refetched}, nil
}

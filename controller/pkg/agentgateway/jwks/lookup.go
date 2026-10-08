package jwks

import (
	"errors"
	"fmt"

	"istio.io/istio/pkg/kube/krt"

	"github.com/agentgateway/agentgateway/controller/pkg/agentgateway/remotehttp"
)

type Lookup interface {
	// InlineForOwner returns the JWKS document fetched for the owner's remote
	// source and the request key it is fetched under. The key is known as soon as
	// the owner resolves, before the first fetch succeeds, so the data plane can
	// ask for the keys it misses (JwksRefresh) from the start.
	InlineForOwner(krtctx krt.HandlerContext, owner RemoteJwksOwner) (string, remotehttp.FetchKey, error)
}

type lookup struct {
	owners krt.Collection[ResolvedOwner]
	cache  *keysetCache
}

func NewLookup(persisted *PersistedEntries, owners krt.Collection[ResolvedOwner]) Lookup {
	return &lookup{
		owners: owners,
		cache:  newKeysetCache(persisted),
	}
}

func (l *lookup) InlineForOwner(krtctx krt.HandlerContext, owner RemoteJwksOwner) (string, remotehttp.FetchKey, error) {
	if l.cache == nil {
		return "", "", fmt.Errorf("jwks persisted cache is not configured")
	}

	resolved := krt.FetchOne(krtctx, l.owners, krt.FilterKey(owner.ResourceName()))
	if resolved == nil {
		return "", "", fmt.Errorf("jwks resolution for %q isn't available", owner.ResourceName())
	}
	if resolved.Error != "" {
		return "", "", errors.New(resolved.Error)
	}

	requestKey := resolved.Source.RequestKey
	keyset, ok := l.cache.Get(krtctx, requestKey)
	if !ok {
		return "", requestKey, fmt.Errorf("jwks keyset for %q isn't available (not yet fetched or fetch failed)", resolved.Source.Target.URL)
	}
	return keyset.JwksJSON, requestKey, nil
}

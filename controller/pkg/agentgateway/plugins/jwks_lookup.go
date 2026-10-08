package plugins

import (
	"fmt"

	"github.com/agentgateway/agentgateway/controller/pkg/agentgateway/jwks"
	"github.com/agentgateway/agentgateway/controller/pkg/agentgateway/remotehttp"
)

// resolveJWKSInlineForOwner returns the inline JWKS for a remote source and the
// request key the control plane fetches it under, which the data plane quotes
// when it asks for a key id the inline set does not have.
func resolveJWKSInlineForOwner(ctx PolicyCtx, owner jwks.RemoteJwksOwner) (string, remotehttp.FetchKey, error) {
	if ctx.JWKSLookup == nil {
		return `{"keys":[]}`, "", fmt.Errorf("jwks lookup is not configured")
	}
	inline, requestKey, err := ctx.JWKSLookup.InlineForOwner(ctx.Krt, owner)
	if err != nil {
		// Keep authentication installed with no trusted keys while reporting the lookup failure.
		// The request key, known once the owner resolved, lets the data plane ask for the keys.
		return `{"keys":[]}`, requestKey, err
	}
	return inline, requestKey, nil
}

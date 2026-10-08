package setup

import (
	"context"
	"crypto/tls"
	"log/slog"
	"math"
	"net"

	envoy_service_discovery_v3 "github.com/envoyproxy/go-control-plane/envoy/service/discovery/v3"
	grpc_middleware "github.com/grpc-ecosystem/go-grpc-middleware"
	grpc_zap "github.com/grpc-ecosystem/go-grpc-middleware/logging/zap"
	"go.uber.org/zap"
	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/credentials"
	"google.golang.org/grpc/reflection"
	"google.golang.org/grpc/status"
	"istio.io/istio/pkg/security"

	"github.com/agentgateway/agentgateway/api"
	"github.com/agentgateway/agentgateway/controller/pkg/agentgateway/jwks"
	"github.com/agentgateway/agentgateway/controller/pkg/metrics"
	"github.com/agentgateway/agentgateway/controller/pkg/syncer/krtxds"
	"github.com/agentgateway/agentgateway/controller/pkg/syncer/nack"
)

const (
	xdsSubsystem = "xds"
)

var (
	xdsAuthRequestTotal = metrics.NewCounter(
		metrics.CounterOpts{
			Subsystem: xdsSubsystem,
			Name:      "auth_rq_total",
			Help:      "Total number of xDS auth requests",
		}, nil)

	xdsAuthSuccessTotal = metrics.NewCounter(
		metrics.CounterOpts{
			Subsystem: xdsSubsystem,
			Name:      "auth_rq_success_total",
			Help:      "Total number of successful xDS auth requests",
		}, nil)

	xdsAuthFailureTotal = metrics.NewCounter(
		metrics.CounterOpts{
			Subsystem: xdsSubsystem,
			Name:      "auth_rq_failure_total",
			Help:      "Total number of failed xDS auth requests",
		}, nil)
)

type certificateProvider interface {
	GetCertificate(*tls.ClientHelloInfo) (*tls.Certificate, error)
}

func runXDSServer(
	ctx context.Context,
	lis net.Listener,
	authenticators []security.Authenticator,
	xdsAuth bool,
	certProvider certificateProvider,
	nackPublisher *nack.Publisher,
	jwksRefresh *jwks.RefreshService,
	reg ...krtxds.Registration,
) {
	baseLogger := slog.Default().With("component", "agentgateway-controlplane")

	serverOpts := getGRPCServerOpts(authenticators, xdsAuth, certProvider, baseLogger)
	grpcServer := grpc.NewServer(serverOpts...)

	ds := krtxds.NewDiscoveryServer(nil, nackPublisher, reg...)
	stop := make(chan struct{})
	context.AfterFunc(ctx, func() {
		close(stop)
	})
	ds.Start(stop)

	reflection.Register(grpcServer)
	envoy_service_discovery_v3.RegisterAggregatedDiscoveryServiceServer(grpcServer, ds)
	if jwksRefresh != nil {
		// The data plane asks for a JWKS refetch on the xDS connection it holds already,
		// under the same authentication as the discovery stream.
		api.RegisterJwksRefreshServer(grpcServer, jwksRefresh)
	}

	baseLogger.Info("starting server", "address", lis.Addr().String())
	go grpcServer.Serve(lis)

	go func() {
		<-ctx.Done()
		grpcServer.GracefulStop()
	}()
}

func getGRPCServerOpts(
	authenticators []security.Authenticator,
	xdsAuth bool,
	certProvider certificateProvider,
	logger *slog.Logger,
) []grpc.ServerOption {
	opts := []grpc.ServerOption{
		grpc.MaxRecvMsgSize(math.MaxInt32),
		grpc.StreamInterceptor(
			grpc_middleware.ChainStreamServer(
				grpc_zap.StreamServerInterceptor(zap.NewNop()),
				func(srv any, ss grpc.ServerStream, info *grpc.StreamServerInfo, handler grpc.StreamHandler) error {
					slog.Debug("gRPC call", "method", info.FullMethod)
					ctx, err := authenticatePeer(ss.Context(), authenticators, xdsAuth)
					if err != nil {
						return err
					}
					return handler(srv, &grpc_middleware.WrappedServerStream{
						ServerStream:   ss,
						WrappedContext: ctx,
					})
				},
			)),
		// Unary calls (JwksRefresh) pass the same authentication as the discovery stream.
		grpc.UnaryInterceptor(
			func(ctx context.Context, req any, info *grpc.UnaryServerInfo, handler grpc.UnaryHandler) (any, error) {
				slog.Debug("gRPC call", "method", info.FullMethod)
				ctx, err := authenticatePeer(ctx, authenticators, xdsAuth)
				if err != nil {
					return nil, err
				}
				return handler(ctx, req)
			}),
	}

	// Add TLS credentials if the certificate watcher was provided. Needed to react to
	// certificate rotations to ensure we're always serving the latest CA certificate.
	if certProvider != nil {
		creds := credentials.NewTLS(&tls.Config{
			MinVersion:     tls.VersionTLS12,
			GetCertificate: certProvider.GetCertificate,
		})
		opts = append(opts, grpc.Creds(creds))
		logger.Info("TLS enabled for xDS servers with certificate watcher")
	} else {
		logger.Warn("TLS disabled for xDS servers: connections will be unencrypted")
	}

	return opts
}

// authenticatePeer runs the xDS authenticators over a call's context and returns
// the context carrying the authenticated peer, or the error to answer the call
// with. With xDS authentication off every call passes and carries no peer.
func authenticatePeer(ctx context.Context, authenticators []security.Authenticator, xdsAuth bool) (context.Context, error) {
	if !xdsAuth {
		slog.Warn("xDS authentication is disabled")
		return ctx, nil
	}
	xdsAuthRequestTotal.Inc()
	am := authenticationManager{
		Authenticators: authenticators,
	}
	if u := am.authenticate(ctx); u != nil {
		xdsAuthSuccessTotal.Inc()
		return context.WithValue(ctx, krtxds.PeerCtxKey, u), nil
	}
	xdsAuthFailureTotal.Inc()
	slog.Error("authentication failed", "reasons", am.authFailMsgs)
	return nil, status.Errorf(codes.Unauthenticated, "authentication failed: %v", am.authFailMsgs)
}

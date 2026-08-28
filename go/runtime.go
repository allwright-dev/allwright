package allwright

import (
	"context"
	"fmt"
	"os"
	"strings"
	"sync"

	enginev1 "allwright.dev/gen/allwright/engine/v1"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

const (
	defaultServerAddr = "127.0.0.1:50051"
	serverAddrEnvVar  = "ALLWRIGHT_SERVER_ADDR"
)

var runtimeState struct {
	mu                 sync.Mutex
	client             *runtimeClient
	serverAddrOverride string
}

func Ping(ctx context.Context) (string, error) {
	runtime, err := getRuntime(ctx)
	if err != nil {
		return "", err
	}

	response, err := runtime.engine.Ping(ctx, &enginev1.PingRequest{})
	if err != nil {
		return "", fmt.Errorf("ping engine server: %w", err)
	}
	return response.GetMessage(), nil
}

func Shutdown() error {
	runtimeState.mu.Lock()
	defer runtimeState.mu.Unlock()

	if runtimeState.client == nil {
		return nil
	}

	err := runtimeState.client.conn.Close()
	runtimeState.client = nil
	shutdownManagedServer()
	return err
}

func SetServerAddr(serverAddr string) error {
	runtimeState.mu.Lock()
	defer runtimeState.mu.Unlock()

	runtimeState.serverAddrOverride = strings.TrimSpace(serverAddr)
	if runtimeState.client == nil {
		return nil
	}

	err := runtimeState.client.conn.Close()
	runtimeState.client = nil
	shutdownManagedServer()
	return err
}

func getRuntime(ctx context.Context) (*runtimeClient, error) {
	runtimeState.mu.Lock()
	defer runtimeState.mu.Unlock()

	if runtimeState.client != nil {
		return runtimeState.client, nil
	}

	serverAddr := resolveServerAddr()
	resolvedServerAddr, err := ensureRuntimeReady(ctx, serverAddr)
	if err != nil {
		return nil, err
	}

	conn, err := grpc.NewClient(resolvedServerAddr, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		return nil, fmt.Errorf("dial engine server at %s: %w", resolvedServerAddr, err)
	}

	runtimeState.client = &runtimeClient{
		conn:   conn,
		engine: enginev1.NewEngineServiceClient(conn),
	}
	return runtimeState.client, nil
}

func resolveServerAddr() string {
	if strings.TrimSpace(runtimeState.serverAddrOverride) != "" {
		return runtimeState.serverAddrOverride
	}

	serverAddr := strings.TrimSpace(os.Getenv(serverAddrEnvVar))
	if serverAddr == "" {
		return defaultServerAddr
	}
	return serverAddr
}

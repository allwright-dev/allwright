package allwright

import (
	"archive/tar"
	"archive/zip"
	"bytes"
	"compress/gzip"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"sync"
	"time"

	enginev1 "allwright.dev/gen/allwright/engine/v1"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

const (
	allwrightAutoInstallEnvVar = "ALLWRIGHT_AUTO_INSTALL"
	allwrightCLIPathEnvVar     = "ALLWRIGHT_CLI_PATH"
	allwrightHomeEnvVar        = "ALLWRIGHT_HOME"
	allwrightRepositoryEnvVar  = "ALLWRIGHT_REPOSITORY"
	allwrightVersionEnvVar     = "ALLWRIGHT_VERSION"
	defaultReleaseRepository   = "allwright-dev/allwright"
	defaultReleaseVersion      = "0.0.42"
)

var bootstrapState struct {
	mu                   sync.Mutex
	managedServerBase    string
	managedServerAddr    string
	managedServer        *exec.Cmd
	managedServerStdout  *tailBuffer
	managedServerStderr  *tailBuffer
	managedServerExited  bool
	managedServerExitErr error
}

type pingStatus struct {
	version string
}

type tailBuffer struct {
	mu   sync.Mutex
	data []byte
}

func (b *tailBuffer) Write(p []byte) (int, error) {
	b.mu.Lock()
	defer b.mu.Unlock()

	const maxBytes = 8_000
	if len(p) >= maxBytes {
		b.data = append([]byte(nil), p[len(p)-maxBytes:]...)
		return len(p), nil
	}

	combined := append(b.data, p...)
	if len(combined) > maxBytes {
		combined = combined[len(combined)-maxBytes:]
	}
	b.data = append([]byte(nil), combined...)
	return len(p), nil
}

func (b *tailBuffer) String() string {
	b.mu.Lock()
	defer b.mu.Unlock()
	return strings.TrimSpace(string(b.data))
}

func ensureRuntimeReady(ctx context.Context, serverAddr string) (string, error) {
	expectedVersion := expectedRuntimeVersion()
	if status, err := pingServer(ctx, serverAddr); err == nil {
		if status.version == expectedVersion {
			return serverAddr, nil
		}
		if !isLocalServerAddr(serverAddr) {
			return "", fmt.Errorf(
				"allwright server at %s is running version %s but this client expects %s",
				serverAddr,
				displayVersion(status.version),
				expectedVersion,
			)
		}
	}
	if !isLocalServerAddr(serverAddr) {
		return "", fmt.Errorf("allwright could not reach engine server at %s; automatic startup is only supported for local addresses", serverAddr)
	}

	var managedServerAddr string
	bootstrapState.mu.Lock()
	if bootstrapState.managedServer != nil && bootstrapState.managedServer.ProcessState == nil && bootstrapState.managedServerBase == serverAddr {
		managedServerAddr = bootstrapState.managedServerAddr
	}
	if managedServerAddr == "" && bootstrapState.managedServer != nil && bootstrapState.managedServer.Process != nil {
		_ = bootstrapState.managedServer.Process.Kill()
		bootstrapState.managedServer = nil
		bootstrapState.managedServerAddr = ""
		bootstrapState.managedServerBase = ""
		bootstrapState.managedServerStdout = nil
		bootstrapState.managedServerStderr = nil
		bootstrapState.managedServerExited = false
		bootstrapState.managedServerExitErr = nil
	}
	bootstrapState.mu.Unlock()

	if managedServerAddr != "" {
		return waitForServer(ctx, managedServerAddr, expectedVersion)
	}

	cliPath, err := ensureCLIAvailable(expectedVersion)
	if err != nil {
		return "", err
	}
	if err := ensureWebPlugin(cliPath, expectedVersion); err != nil {
		return "", err
	}

	resolvedServerAddr := serverAddr
	if status, err := pingServer(ctx, serverAddr); err == nil && status.version != expectedVersion {
		resolvedServerAddr, err = allocateManagedServerAddr(serverAddr)
		if err != nil {
			return "", err
		}
	}

	cmd := exec.Command(cliPath, "serve", "--listen-addr", cliListenAddr(resolvedServerAddr))
	cmd.Stdin = nil
	stdoutBuf := &tailBuffer{}
	stderrBuf := &tailBuffer{}
	cmd.Stdout = stdoutBuf
	cmd.Stderr = stderrBuf
	if err := cmd.Start(); err != nil {
		return "", fmt.Errorf("start allwright server with %s: %w", cliPath, err)
	}

	bootstrapState.mu.Lock()
	bootstrapState.managedServerBase = serverAddr
	bootstrapState.managedServer = cmd
	bootstrapState.managedServerAddr = resolvedServerAddr
	bootstrapState.managedServerStdout = stdoutBuf
	bootstrapState.managedServerStderr = stderrBuf
	bootstrapState.managedServerExited = false
	bootstrapState.managedServerExitErr = nil
	bootstrapState.mu.Unlock()

	go func(cmd *exec.Cmd) {
		err := cmd.Wait()
		bootstrapState.mu.Lock()
		defer bootstrapState.mu.Unlock()
		if bootstrapState.managedServer == cmd {
			bootstrapState.managedServerExited = true
			bootstrapState.managedServerExitErr = err
		}
	}(cmd)

	return waitForServer(ctx, resolvedServerAddr, expectedVersion)
}

func shutdownManagedServer() {
	bootstrapState.mu.Lock()
	cmd := bootstrapState.managedServer
	exited := bootstrapState.managedServerExited
	bootstrapState.managedServerBase = ""
	bootstrapState.managedServer = nil
	bootstrapState.managedServerAddr = ""
	bootstrapState.managedServerStdout = nil
	bootstrapState.managedServerStderr = nil
	bootstrapState.managedServerExited = false
	bootstrapState.managedServerExitErr = nil
	bootstrapState.mu.Unlock()

	if cmd != nil && !exited && cmd.Process != nil {
		_ = cmd.Process.Kill()
	}
}

func waitForServer(ctx context.Context, serverAddr string, expectedVersion string) (string, error) {
	deadline := time.Now().Add(20 * time.Second)
	for time.Now().Before(deadline) {
		if details := managedServerFailureDetails(); details != "" {
			shutdownManagedServer()
			return "", fmt.Errorf("allwright server exited before becoming ready at %s\n%s", serverAddr, details)
		}
		if status, err := pingServer(ctx, serverAddr); err == nil && status.version == expectedVersion {
			return serverAddr, nil
		}
		time.Sleep(250 * time.Millisecond)
	}
	details := managedServerFailureDetails()
	shutdownManagedServer()
	if details != "" {
		return "", fmt.Errorf(
			"timed out waiting for allwright server at %s to become ready with version %s\n%s",
			serverAddr,
			expectedVersion,
			details,
		)
	}
	return "", fmt.Errorf("timed out waiting for allwright server at %s to become ready with version %s", serverAddr, expectedVersion)
}

func pingServer(ctx context.Context, serverAddr string) (*pingStatus, error) {
	timeoutCtx, cancel := context.WithTimeout(ctx, time.Second)
	defer cancel()

	conn, err := grpc.NewClient(serverAddr, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		return nil, err
	}
	defer conn.Close()

	response, err := enginev1.NewEngineServiceClient(conn).Ping(timeoutCtx, &enginev1.PingRequest{})
	if err != nil {
		return nil, err
	}
	return &pingStatus{version: normalizeReleaseVersion(response.GetVersion())}, nil
}

func ensureCLIAvailable(expectedVersion string) (string, error) {
	if cliPath := strings.TrimSpace(os.Getenv(allwrightCLIPathEnvVar)); cliPath != "" {
		if isFile(cliPath) {
			matches, err := cliVersionMatches(cliPath, expectedVersion)
			if err != nil {
				return "", err
			}
			if matches {
				return cliPath, nil
			}
		}
	}

	home, err := allwrightHome()
	if err != nil {
		return "", err
	}
	bundled := filepath.Join(home, "bin", cliFilename())
	if isFile(bundled) {
		matches, err := cliVersionMatches(bundled, expectedVersion)
		if err != nil {
			return "", err
		}
		if matches {
			return bundled, nil
		}
	}
	if cliPath, ok := repoLocalCLIPath(); ok {
		matches, err := cliVersionMatches(cliPath, expectedVersion)
		if err != nil {
			return "", err
		}
		if matches {
			return cliPath, nil
		}
	}
	if cliPath, err := exec.LookPath(cliFilename()); err == nil {
		matches, err := cliVersionMatches(cliPath, expectedVersion)
		if err != nil {
			return "", err
		}
		if matches {
			return cliPath, nil
		}
	}
	if !autoInstallEnabled() {
		return "", fmt.Errorf("allwright CLI was not found; install it first or set %s", allwrightCLIPathEnvVar)
	}
	return installCLI()
}

func installCLI() (string, error) {
	home, err := allwrightHome()
	if err != nil {
		return "", err
	}
	installDir := filepath.Join(home, "bin")
	if err := os.MkdirAll(installDir, 0o755); err != nil {
		return "", fmt.Errorf("create allwright CLI install dir %s: %w", installDir, err)
	}
	cliPath := filepath.Join(installDir, cliFilename())
	tag, err := resolveReleaseTag()
	if err != nil {
		return "", err
	}
	assetName, err := cliAssetName(tag)
	if err != nil {
		return "", err
	}
	assetBytes, err := downloadReleaseAsset(tag, assetName)
	if err != nil {
		return "", err
	}
	if err := unpackCLIArchive(assetName, assetBytes, cliPath); err != nil {
		return "", err
	}
	return cliPath, nil
}

func ensureWebPlugin(cliPath string, expectedVersion string) error {
	home, err := allwrightHome()
	if err != nil {
		return err
	}
	pluginPath := filepath.Join(home, "plugins", "web", "lib", webPluginFilename())
	if isFile(pluginPath) {
		if version, err := installedPluginVersion(home, "web"); err == nil && version == expectedVersion {
			return nil
		}
	}

	if expectedVersion == "" {
		expectedVersion = normalizeReleaseVersion(defaultReleaseVersion)
	}

	cmd := exec.Command(cliPath, "plugin", "install", "web", "--version", expectedVersion)
	cmd.Stdout = io.Discard
	cmd.Stderr = io.Discard
	if err := cmd.Run(); err != nil {
		return fmt.Errorf("install allwright web plugin: %w", err)
	}
	if !isFile(pluginPath) {
		return fmt.Errorf("allwright attempted to install the web plugin automatically, but the runtime library is still missing")
	}
	if version, err := installedPluginVersion(home, "web"); err == nil && version == expectedVersion {
		return nil
	}
	return fmt.Errorf("allwright attempted to install the web plugin automatically, but version %s is still not active", expectedVersion)
}

func resolveReleaseTag() (string, error) {
	version := strings.TrimSpace(os.Getenv(allwrightVersionEnvVar))
	if version == "" {
		version = defaultReleaseVersion
	}
	if version != "latest" {
		return normalizeReleaseTag(version), nil
	}

	repository := strings.TrimSpace(os.Getenv(allwrightRepositoryEnvVar))
	if repository == "" {
		repository = defaultReleaseRepository
	}
	request, err := http.NewRequest(http.MethodGet, fmt.Sprintf("https://api.github.com/repos/%s/releases/latest", repository), nil)
	if err != nil {
		return "", err
	}
	request.Header.Set("User-Agent", "allwright-go/"+defaultReleaseVersion)
	response, err := http.DefaultClient.Do(request)
	if err != nil {
		return "", err
	}
	defer response.Body.Close()
	if response.StatusCode < 200 || response.StatusCode >= 300 {
		return "", fmt.Errorf("resolve latest allwright release: %s", response.Status)
	}
	var payload struct {
		TagName string `json:"tag_name"`
	}
	if err := json.NewDecoder(response.Body).Decode(&payload); err != nil {
		return "", err
	}
	if strings.TrimSpace(payload.TagName) == "" {
		return "", fmt.Errorf("latest allwright release metadata did not include tag_name")
	}
	return payload.TagName, nil
}

func cliAssetName(tag string) (string, error) {
	target := ""
	switch runtime.GOOS + "/" + runtime.GOARCH {
	case "darwin/arm64":
		target = "aarch64-apple-darwin"
	case "darwin/amd64":
		target = "x86_64-apple-darwin"
	case "linux/arm64":
		target = "aarch64-unknown-linux-gnu"
	case "linux/amd64":
		target = "x86_64-unknown-linux-gnu"
	case "windows/arm64":
		target = "aarch64-pc-windows-msvc"
	case "windows/amd64":
		target = "x86_64-pc-windows-msvc"
	default:
		return "", fmt.Errorf("automatic allwright CLI install is not supported on %s/%s", runtime.GOOS, runtime.GOARCH)
	}
	ext := "tar.gz"
	if runtime.GOOS == "windows" {
		ext = "zip"
	}
	return fmt.Sprintf("allwright-%s-%s.%s", tag, target, ext), nil
}

func downloadReleaseAsset(tag, assetName string) ([]byte, error) {
	repository := strings.TrimSpace(os.Getenv(allwrightRepositoryEnvVar))
	if repository == "" {
		repository = defaultReleaseRepository
	}
	request, err := http.NewRequest(http.MethodGet, fmt.Sprintf("https://github.com/%s/releases/download/%s/%s", repository, tag, assetName), nil)
	if err != nil {
		return nil, err
	}
	request.Header.Set("User-Agent", "allwright-go/"+defaultReleaseVersion)
	response, err := http.DefaultClient.Do(request)
	if err != nil {
		return nil, err
	}
	defer response.Body.Close()
	if response.StatusCode < 200 || response.StatusCode >= 300 {
		return nil, fmt.Errorf("download allwright CLI asset %s: %s", assetName, response.Status)
	}
	return io.ReadAll(response.Body)
}

func unpackCLIArchive(assetName string, assetBytes []byte, destination string) error {
	if strings.HasSuffix(assetName, ".zip") {
		archive, err := zip.NewReader(bytes.NewReader(assetBytes), int64(len(assetBytes)))
		if err != nil {
			return err
		}
		for _, file := range archive.File {
			if file.Name != "bin/"+cliFilename() {
				continue
			}
			reader, err := file.Open()
			if err != nil {
				return err
			}
			defer reader.Close()
			output, err := os.Create(destination)
			if err != nil {
				return err
			}
			defer output.Close()
			if _, err := io.Copy(output, reader); err != nil {
				return err
			}
			return os.Chmod(destination, 0o755)
		}
		return fmt.Errorf("allwright CLI archive did not contain bin/%s", cliFilename())
	}

	gzipReader, err := gzip.NewReader(bytes.NewReader(assetBytes))
	if err != nil {
		return err
	}
	defer gzipReader.Close()
	archive := tar.NewReader(gzipReader)
	for {
		header, err := archive.Next()
		if err == io.EOF {
			break
		}
		if err != nil {
			return err
		}
		if header.Name != "bin/"+cliFilename() {
			continue
		}
		output, err := os.Create(destination)
		if err != nil {
			return err
		}
		defer output.Close()
		if _, err := io.Copy(output, archive); err != nil {
			return err
		}
		return os.Chmod(destination, 0o755)
	}
	return fmt.Errorf("allwright CLI archive did not contain bin/%s", cliFilename())
}

func allwrightHome() (string, error) {
	if home := strings.TrimSpace(os.Getenv(allwrightHomeEnvVar)); home != "" {
		return home, nil
	}
	home, err := os.UserHomeDir()
	if err != nil {
		return "", fmt.Errorf("resolve ALLWRIGHT_HOME: %w", err)
	}
	return filepath.Join(home, ".allwright"), nil
}

func autoInstallEnabled() bool {
	switch strings.ToLower(strings.TrimSpace(os.Getenv(allwrightAutoInstallEnvVar))) {
	case "0", "false", "no":
		return false
	default:
		return true
	}
}

func cliListenAddr(serverAddr string) string {
	return strings.TrimPrefix(strings.TrimPrefix(serverAddr, "http://"), "https://")
}

func normalizeReleaseTag(version string) string {
	version = strings.TrimSpace(version)
	if strings.HasPrefix(version, "v") {
		return version
	}
	return "v" + version
}

func normalizeReleaseVersion(version string) string {
	return strings.TrimPrefix(strings.TrimSpace(version), "v")
}

func expectedRuntimeVersion() string {
	version := strings.TrimSpace(os.Getenv(allwrightVersionEnvVar))
	if version == "" {
		version = defaultReleaseVersion
	}
	return normalizeReleaseVersion(version)
}

func isLocalServerAddr(serverAddr string) bool {
	trimmed := cliListenAddr(serverAddr)
	host := trimmed
	if strings.LastIndex(host, ":") > 0 {
		host = host[:strings.LastIndex(host, ":")]
	}
	host = strings.Trim(host, "[]")
	return host == "127.0.0.1" || host == "localhost" || host == "::1"
}

func installedPluginVersion(home string, pluginID string) (string, error) {
	manifestPath := filepath.Join(home, "plugins.txt")
	contents, err := os.ReadFile(manifestPath)
	if err != nil {
		if os.IsNotExist(err) {
			return "", nil
		}
		return "", err
	}
	for _, line := range strings.Split(string(contents), "\n") {
		trimmed := strings.TrimSpace(line)
		if trimmed == "" || strings.HasPrefix(trimmed, "#") {
			continue
		}
		parts := strings.SplitN(trimmed, "\t", 3)
		if len(parts) < 3 || parts[0] != pluginID {
			continue
		}
		return normalizeReleaseVersion(parts[2]), nil
	}
	return "", nil
}

func allocateManagedServerAddr(serverAddr string) (string, error) {
	host := localBindingHost(serverAddr)
	listener, err := net.Listen("tcp", net.JoinHostPort(host, "0"))
	if err != nil {
		return "", fmt.Errorf("reserve local port for managed allwright server on %s: %w", host, err)
	}
	port := listener.Addr().(*net.TCPAddr).Port
	_ = listener.Close()
	return net.JoinHostPort(host, fmt.Sprintf("%d", port)), nil
}

func repoLocalCLIPath() (string, bool) {
	_, currentFile, _, ok := runtime.Caller(0)
	if !ok {
		return "", false
	}
	repoRoot := filepath.Dir(filepath.Dir(currentFile))
	for _, candidate := range []string{
		filepath.Join(repoRoot, "target", "debug", cliFilename()),
		filepath.Join(repoRoot, "target", "release", cliFilename()),
	} {
		if isFile(candidate) {
			return candidate, true
		}
	}
	return "", false
}

func localBindingHost(serverAddr string) string {
	trimmed := cliListenAddr(serverAddr)
	host := trimmed
	if strings.LastIndex(host, ":") > 0 {
		host = host[:strings.LastIndex(host, ":")]
	}
	host = strings.Trim(host, "[]")
	if host == "::1" {
		return "::1"
	}
	return "127.0.0.1"
}

func displayVersion(version string) string {
	if strings.TrimSpace(version) == "" {
		return "unknown"
	}
	return version
}

func managedServerFailureDetails() string {
	bootstrapState.mu.Lock()
	defer bootstrapState.mu.Unlock()

	if !bootstrapState.managedServerExited && bootstrapState.managedServerExitErr == nil {
		return ""
	}

	parts := make([]string, 0, 3)
	if bootstrapState.managedServerExitErr != nil {
		parts = append(parts, fmt.Sprintf("exit error: %v", bootstrapState.managedServerExitErr))
	}
	if bootstrapState.managedServerStdout != nil {
		if stdout := bootstrapState.managedServerStdout.String(); stdout != "" {
			parts = append(parts, "stdout:\n"+stdout)
		}
	}
	if bootstrapState.managedServerStderr != nil {
		if stderr := bootstrapState.managedServerStderr.String(); stderr != "" {
			parts = append(parts, "stderr:\n"+stderr)
		}
	}
	return strings.Join(parts, "\n")
}

func cliVersionMatches(cliPath string, expectedVersion string) (bool, error) {
	output, err := exec.Command(cliPath, "--version").Output()
	if err != nil {
		return false, fmt.Errorf("inspect allwright CLI version via %s: %w", cliPath, err)
	}
	for _, token := range strings.Fields(string(output)) {
		if token != "" && token[0] >= '0' && token[0] <= '9' {
			return normalizeReleaseVersion(token) == expectedVersion, nil
		}
	}
	return false, nil
}

func cliFilename() string {
	if runtime.GOOS == "windows" {
		return "allwright.exe"
	}
	return "allwright"
}

func webPluginFilename() string {
	switch runtime.GOOS {
	case "darwin":
		return "liballwright_surface_web.dylib"
	case "linux":
		return "liballwright_surface_web.so"
	case "windows":
		return "allwright_surface_web.dll"
	default:
		return "allwright_surface_web.unknown"
	}
}

func isFile(path string) bool {
	info, err := os.Stat(path)
	return err == nil && !info.IsDir()
}

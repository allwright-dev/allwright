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
	defaultReleaseVersion      = "0.0.7"
)

var bootstrapState struct {
	mu                sync.Mutex
	managedServerAddr string
	managedServer     *exec.Cmd
}

func ensureRuntimeReady(ctx context.Context, serverAddr string) error {
	if pingServer(ctx, serverAddr) == nil {
		return nil
	}
	if !isLocalServerAddr(serverAddr) {
		return fmt.Errorf("allwright could not reach engine server at %s; automatic startup is only supported for local addresses", serverAddr)
	}

	bootstrapState.mu.Lock()
	defer bootstrapState.mu.Unlock()

	if bootstrapState.managedServer != nil && bootstrapState.managedServer.ProcessState == nil && bootstrapState.managedServerAddr == serverAddr {
		return waitForServer(ctx, serverAddr)
	}

	cliPath, err := ensureCLIAvailable()
	if err != nil {
		return err
	}
	if err := ensureWebPlugin(cliPath); err != nil {
		return err
	}

	cmd := exec.Command(cliPath, "serve", "--listen-addr", cliListenAddr(serverAddr))
	cmd.Stdin = nil
	cmd.Stdout = io.Discard
	cmd.Stderr = io.Discard
	if err := cmd.Start(); err != nil {
		return fmt.Errorf("start allwright server with %s: %w", cliPath, err)
	}

	bootstrapState.managedServer = cmd
	bootstrapState.managedServerAddr = serverAddr
	return waitForServer(ctx, serverAddr)
}

func shutdownManagedServer() {
	bootstrapState.mu.Lock()
	defer bootstrapState.mu.Unlock()

	if bootstrapState.managedServer != nil && bootstrapState.managedServer.Process != nil {
		_ = bootstrapState.managedServer.Process.Kill()
		_, _ = bootstrapState.managedServer.Process.Wait()
	}
	bootstrapState.managedServer = nil
	bootstrapState.managedServerAddr = ""
}

func waitForServer(ctx context.Context, serverAddr string) error {
	deadline := time.Now().Add(20 * time.Second)
	for time.Now().Before(deadline) {
		if pingServer(ctx, serverAddr) == nil {
			return nil
		}
		time.Sleep(250 * time.Millisecond)
	}
	shutdownManagedServer()
	return fmt.Errorf("timed out waiting for allwright server at %s to become ready", serverAddr)
}

func pingServer(ctx context.Context, serverAddr string) error {
	timeoutCtx, cancel := context.WithTimeout(ctx, time.Second)
	defer cancel()

	conn, err := grpc.NewClient(serverAddr, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		return err
	}
	defer conn.Close()

	_, err = enginev1.NewEngineServiceClient(conn).Ping(timeoutCtx, &enginev1.PingRequest{})
	return err
}

func ensureCLIAvailable() (string, error) {
	if cliPath := strings.TrimSpace(os.Getenv(allwrightCLIPathEnvVar)); cliPath != "" {
		if isFile(cliPath) {
			return cliPath, nil
		}
	}

	home, err := allwrightHome()
	if err != nil {
		return "", err
	}
	bundled := filepath.Join(home, "bin", cliFilename())
	if isFile(bundled) {
		return bundled, nil
	}
	if cliPath, err := exec.LookPath(cliFilename()); err == nil {
		return cliPath, nil
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

func ensureWebPlugin(cliPath string) error {
	home, err := allwrightHome()
	if err != nil {
		return err
	}
	pluginPath := filepath.Join(home, "plugins", "web", "lib", webPluginFilename())
	if isFile(pluginPath) {
		return nil
	}

	version := strings.TrimSpace(os.Getenv(allwrightVersionEnvVar))
	if version == "" {
		version = defaultReleaseVersion
	}
	cmd := exec.Command(cliPath, "plugin", "install", "web", "--version", normalizeReleaseVersion(version))
	cmd.Stdout = io.Discard
	cmd.Stderr = io.Discard
	if err := cmd.Run(); err != nil {
		return fmt.Errorf("install allwright web plugin: %w", err)
	}
	if !isFile(pluginPath) {
		return fmt.Errorf("allwright attempted to install the web plugin automatically, but the runtime library is still missing")
	}
	return nil
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

func isLocalServerAddr(serverAddr string) bool {
	trimmed := cliListenAddr(serverAddr)
	host := trimmed
	if strings.LastIndex(host, ":") > 0 {
		host = host[:strings.LastIndex(host, ":")]
	}
	host = strings.Trim(host, "[]")
	return host == "127.0.0.1" || host == "localhost" || host == "::1"
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

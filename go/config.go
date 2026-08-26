package allwright

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"

	"gopkg.in/yaml.v3"
)

var configFilenames = []string{
	"allwright.config.yaml",
	"allwright.config.yml",
	"allwright.config.json",
	".allwright/config.yaml",
	".allwright/config.yml",
	".allwright/config.json",
}

func FindConfigFile(startDir string) (string, error) {
	currentDir := startDir
	if strings.TrimSpace(currentDir) == "" {
		var err error
		currentDir, err = os.Getwd()
		if err != nil {
			return "", fmt.Errorf("get working directory: %w", err)
		}
	}
	currentDir, _ = filepath.Abs(currentDir)

	for {
		for _, filename := range configFilenames {
			candidate := filepath.Join(currentDir, filename)
			info, err := os.Stat(candidate)
			if err == nil && !info.IsDir() {
				return candidate, nil
			}
		}

		parentDir := filepath.Dir(currentDir)
		if parentDir == currentDir {
			return "", nil
		}
		currentDir = parentDir
	}
}

func LoadConfigFile(path string) (*AllwrightConfig, error) {
	resolved, err := filepath.Abs(path)
	if err != nil {
		return nil, fmt.Errorf("resolve config path: %w", err)
	}

	raw, err := os.ReadFile(resolved)
	if err != nil {
		return nil, fmt.Errorf("read config file %s: %w", resolved, err)
	}

	config := &AllwrightConfig{}
	switch strings.ToLower(filepath.Ext(resolved)) {
	case ".json":
		if err := json.Unmarshal(raw, config); err != nil {
			return nil, fmt.Errorf("parse config file %s as JSON: %w", resolved, err)
		}
	case ".yaml", ".yml":
		if err := yaml.Unmarshal(raw, config); err != nil {
			return nil, fmt.Errorf("parse config file %s as YAML: %w", resolved, err)
		}
	default:
		return nil, fmt.Errorf("unsupported allwright config file extension %q for %s", filepath.Ext(resolved), resolved)
	}

	if err := validateConfig(config, resolved); err != nil {
		return nil, err
	}
	return config, nil
}

func ResolveConfig(options ResolveConfigOptions) (*ResolvedConfig, error) {
	configFile := strings.TrimSpace(options.ConfigFile)
	if configFile == "" {
		resolved, err := FindConfigFile(options.Cwd)
		if err != nil {
			return nil, err
		}
		configFile = resolved
	}

	config := &AllwrightConfig{}
	if configFile != "" {
		loaded, err := LoadConfigFile(configFile)
		if err != nil {
			return nil, err
		}
		config = loaded
	}

	suiteName := strings.TrimSpace(options.Suite)
	var suite *suiteConfig
	if suiteName != "" {
		selected, ok := config.Suites[suiteName]
		if !ok {
			source := configFile
			if source == "" {
				source = "the resolved config file"
			}
			return nil, fmt.Errorf("allwright config suite %q was not found in %s", suiteName, source)
		}
		suite = &selected
	}

	browserName := "chromium"
	if config.Browser != nil && strings.TrimSpace(config.Browser.Name) != "" {
		browserName = strings.TrimSpace(config.Browser.Name)
	}
	if suite != nil && suite.Browser != nil && strings.TrimSpace(suite.Browser.Name) != "" {
		browserName = strings.TrimSpace(suite.Browser.Name)
	}

	browserBinary := firstNonEmpty(
		configBrowserBinaryFromSuite(suite),
		configBrowserBinary(config.Browser),
	)
	serverAddr := firstNonEmpty(
		configServerAddrFromSuite(suite),
		configServerAddr(config.Server),
	)
	launchOptions := mergeLaunchOptions(
		launchOptionsFromConfig(config.Browser),
		launchOptionsFromSuite(suite),
	)
	if browserBinary != "" {
		launchOptions.BrowserBinary = browserBinary
	}
	expect := mergeRetryConfig(config.Expect, suiteExpect(suite))

	return &ResolvedConfig{
		ConfigFilePath: configFile,
		SuiteName:      suiteName,
		ServerAddr:     serverAddr,
		BrowserName:    browserName,
		BrowserBinary:  browserBinary,
		LaunchOptions:  launchOptions,
		Expect:         expect,
	}, nil
}

func validateConfig(config *AllwrightConfig, source string) error {
	if config == nil {
		return nil
	}
	if config.SchemaVersion != 0 && config.SchemaVersion != 1 {
		return fmt.Errorf("allwright config %s has unsupported schemaVersion %d; expected 1", source, config.SchemaVersion)
	}

	if err := validateBrowserName(source, configBrowserName(config.Browser)); err != nil {
		return err
	}
	for suiteName, suite := range config.Suites {
		if err := validateBrowserName(source, configBrowserName(suite.Browser)); err != nil {
			return fmt.Errorf("suite %q: %w", suiteName, err)
		}
	}
	return nil
}

func validateBrowserName(source string, browserName string) error {
	switch strings.ToLower(strings.TrimSpace(browserName)) {
	case "", "chromium", "firefox":
		return nil
	default:
		return fmt.Errorf("allwright config %s has unsupported browser.name %q; use \"chromium\" or \"firefox\"", source, browserName)
	}
}

func configBrowserName(browser *configBrowser) string {
	if browser == nil {
		return ""
	}
	return strings.TrimSpace(browser.Name)
}

func configBrowserBinary(browser *configBrowser) string {
	if browser == nil {
		return ""
	}
	return strings.TrimSpace(browser.Binary)
}

func configBrowserBinaryFromSuite(suite *suiteConfig) string {
	if suite == nil {
		return ""
	}
	return configBrowserBinary(suite.Browser)
}

func configServerAddr(server *configServer) string {
	if server == nil {
		return ""
	}
	return strings.TrimSpace(server.Addr)
}

func configServerAddrFromSuite(suite *suiteConfig) string {
	if suite == nil {
		return ""
	}
	return configServerAddr(suite.Server)
}

func launchOptionsFromConfig(browser *configBrowser) LaunchOptions {
	if browser == nil || browser.LaunchOptions == nil {
		return LaunchOptions{}
	}
	options := LaunchOptions{}
	if strings.TrimSpace(browser.LaunchOptions.BrowserBinary) != "" {
		options.BrowserBinary = strings.TrimSpace(browser.LaunchOptions.BrowserBinary)
	}
	if browser.LaunchOptions.TimeoutMs > 0 {
		options.Timeout = time.Duration(browser.LaunchOptions.TimeoutMs) * time.Millisecond
	}
	return options
}

func launchOptionsFromSuite(suite *suiteConfig) LaunchOptions {
	if suite == nil {
		return LaunchOptions{}
	}
	return launchOptionsFromConfig(suite.Browser)
}

func mergeLaunchOptions(base LaunchOptions, override LaunchOptions) LaunchOptions {
	merged := base
	if strings.TrimSpace(override.BrowserBinary) != "" {
		merged.BrowserBinary = strings.TrimSpace(override.BrowserBinary)
	}
	if override.Timeout > 0 {
		merged.Timeout = override.Timeout
	}
	return merged
}

func suiteExpect(suite *suiteConfig) *RetryConfig {
	if suite == nil {
		return nil
	}
	return suite.Expect
}

func mergeRetryConfig(base *RetryConfig, override *RetryConfig) RetryConfig {
	merged := RetryConfig{}
	if base != nil {
		merged = *base
	}
	if override != nil {
		if override.TimeoutMs > 0 {
			merged.TimeoutMs = override.TimeoutMs
		}
		if override.IntervalMs > 0 {
			merged.IntervalMs = override.IntervalMs
		}
	}
	return merged
}

func firstNonEmpty(values ...string) string {
	for _, value := range values {
		trimmed := strings.TrimSpace(value)
		if trimmed != "" {
			return trimmed
		}
	}
	return ""
}

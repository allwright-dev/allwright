package main

import (
	"bufio"
	"context"
	"flag"
	"fmt"
	"log"
	"os"
	"strings"
	"time"

	allwright "allwright.dev"
)

type browserSessionFlags struct {
	serverAddr    string
	browser       string
	browserBinary string
	navigateURL   string
	clickSelector string
}

func main() {
	serverAddr := flag.String("server-addr", "127.0.0.1:50051", "Engine server address")
	browser := flag.String("browser", "chromium", "Browser backend to launch: chromium or firefox")
	browserBinary := flag.String("browser-binary", "", "Optional browser binary path or executable name")
	navigateURL := flag.String("navigate-url", "https://example.com", "URL to navigate the initial tab to")
	clickSelector := flag.String("click-selector", "", "Optional CSS selector to click over BiDi after navigation")
	flag.Parse()

	if err := os.Setenv("ALLWRIGHT_SERVER_ADDR", *serverAddr); err != nil {
		log.Fatalf("set ALLWRIGHT_SERVER_ADDR: %v", err)
	}
	defer func() {
		if err := allwright.Shutdown(); err != nil {
			log.Printf("shutdown allwright Go client: %v", err)
		}
	}()

	ctx, cancel := context.WithTimeout(context.Background(), 60*time.Second)
	defer cancel()

	runBrowserSession(ctx, browserSessionFlags{
		serverAddr:    *serverAddr,
		browser:       *browser,
		browserBinary: *browserBinary,
		navigateURL:   *navigateURL,
		clickSelector: *clickSelector,
	})
}

func runBrowserSession(ctx context.Context, flags browserSessionFlags) {
	fmt.Printf("[go-playground] launching %s with browser_binary=%q via singleton client runtime\n", flags.browser, flags.browserBinary)
	var (
		browser *allwright.Browser
		err     error
	)
	switch strings.ToLower(strings.TrimSpace(flags.browser)) {
	case "firefox":
		browser, err = allwright.LaunchFirefox(ctx, allwright.LaunchOptions{
			BrowserBinary: flags.browserBinary,
		})
	case "chromium", "chrome":
		browser, err = allwright.LaunchChrome(ctx, allwright.LaunchOptions{
			BrowserBinary: flags.browserBinary,
		})
	default:
		log.Fatalf("unsupported --browser value %q; use chromium or firefox", flags.browser)
	}
	if err != nil {
		log.Fatalf("launch browser: %v", err)
	}

	initialTab := browser.InitialTab()
	logBrowserLaunch(browser, initialTab.SessionID())

	navigateResult, err := initialTab.Navigate(ctx, flags.navigateURL)
	if err != nil {
		log.Fatalf("navigate initial tab: %v", err)
	}
	fmt.Printf(
		"[%s] tab navigated: %s (%s)\n",
		initialTab.SessionID(),
		navigateResult.URL,
		navigateResult.Note,
	)
	fmt.Printf(
		"[%s] automation session: bidi_session_id=%s mapper_target_id=%s mapper_session_id=%s package_version=%s\n",
		initialTab.SessionID(),
		navigateResult.BidiSessionID,
		navigateResult.MapperTargetID,
		navigateResult.MapperSessionID,
		navigateResult.PackageVersion,
	)

	if strings.TrimSpace(flags.clickSelector) != "" {
		clickResult, err := initialTab.Click(ctx, flags.clickSelector)
		if err != nil {
			log.Fatalf("click element: %v", err)
		}
		fmt.Printf(
			"[%s] element clicked: selector=%s (%s) bidi_session_id=%s\n",
			initialTab.SessionID(),
			clickResult.Selector,
			clickResult.Note,
			clickResult.BidiSessionID,
		)
	}

	waitForEnter("[go-playground] Press Enter to close the browser session and keep the browser open for observation...")

	if err := initialTab.Close(ctx); err != nil {
		log.Fatalf("close initial tab: %v", err)
	}
	fmt.Printf("[%s] tab session closed\n", initialTab.SessionID())

	if err := browser.Close(ctx); err != nil {
		log.Fatalf("close browser session: %v", err)
	}
	fmt.Printf("[%s] session closed\n", browser.SessionID())
}

func logBrowserLaunch(browser *allwright.Browser, initialTabSessionID string) {
	if browser.CdpWebSocketURL() == "" {
		fmt.Printf(
			"[%s] browser launched: %s (%s) user_data_dir=%s initial_tab_session_id=%s\n",
			browser.SessionID(),
			browser.BrowserName(),
			browser.LaunchNote(),
			browser.UserDataDir(),
			initialTabSessionID,
		)
		return
	}

	fmt.Printf(
		"[%s] browser launched: %s (%s) cdp=%s user_data_dir=%s initial_tab_session_id=%s\n",
		browser.SessionID(),
		browser.BrowserName(),
		browser.LaunchNote(),
		browser.CdpWebSocketURL(),
		browser.UserDataDir(),
		initialTabSessionID,
	)
}

func waitForEnter(prompt string) {
	fmt.Println(prompt)
	reader := bufio.NewReader(os.Stdin)
	if _, err := reader.ReadString('\n'); err != nil {
		log.Fatalf("read keyboard confirmation: %v", err)
	}
}

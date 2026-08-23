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

	allwright "allwright-go/client"
)

type browserSessionFlags struct {
	serverAddr    string
	chromeBinary  string
	navigateURL   string
	clickSelector string
}

func main() {
	serverAddr := flag.String("server-addr", "127.0.0.1:50051", "Engine server address")
	chromeBinary := flag.String("chrome-binary", "", "Optional Chrome binary path or executable name")
	navigateURL := flag.String("navigate-url", "https://example.com", "URL to navigate the initial tab to")
	clickSelector := flag.String("click-selector", "", "Optional CSS selector to click over BiDi after navigation")
	flag.Parse()

	if flag.NArg() == 0 {
		log.Fatalf("expected a subcommand: ping or browser-session")
	}

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

	switch flag.Arg(0) {
	case "ping":
		runPing(ctx)
	case "browser-session":
		runBrowserSession(ctx, browserSessionFlags{
			serverAddr:    *serverAddr,
			chromeBinary:  *chromeBinary,
			navigateURL:   *navigateURL,
			clickSelector: *clickSelector,
		})
	default:
		log.Fatalf("unsupported subcommand %q", flag.Arg(0))
	}
}

func runPing(ctx context.Context) {
	message, err := allwright.Ping(ctx)
	if err != nil {
		log.Fatalf("ping: %v", err)
	}
	fmt.Printf("ping response: %s\n", message)
}

func runBrowserSession(ctx context.Context, flags browserSessionFlags) {
	fmt.Printf("[go-playground] launching chrome with chrome_binary=%q via singleton client runtime\n", flags.chromeBinary)
	browser, err := allwright.LaunchChrome(ctx, allwright.LaunchOptions{
		ChromeBinary: flags.chromeBinary,
	})
	if err != nil {
		log.Fatalf("launch chrome: %v", err)
	}

	initialTab := browser.InitialTab()
	fmt.Printf(
		"[%s] chrome launched: %s (%s) cdp=%s user_data_dir=%s initial_tab_session_id=%s\n",
		browser.SessionID(),
		browser.BrowserName(),
		browser.LaunchNote(),
		browser.CdpWebSocketURL(),
		browser.UserDataDir(),
		initialTab.SessionID(),
	)

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
		"[%s] chromium-bidi injected: bidi_session_id=%s mapper_target_id=%s mapper_session_id=%s package_version=%s\n",
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

	tabPong, err := initialTab.Ping(ctx, "go-playground-tab-1")
	if err != nil {
		log.Fatalf("ping initial tab: %v", err)
	}
	fmt.Printf("[%s] tab session pong: %s\n", initialTab.SessionID(), tabPong)

	waitForEnter("[go-playground] Press Enter to close the browser session and Chrome...")

	if err := initialTab.Close(ctx); err != nil {
		log.Fatalf("close initial tab: %v", err)
	}
	fmt.Printf("[%s] tab session closed\n", initialTab.SessionID())

	pong, err := browser.Ping(ctx, "go-playground")
	if err != nil {
		log.Fatalf("ping browser session: %v", err)
	}
	fmt.Printf("[%s] session pong: %s\n", browser.SessionID(), pong)

	if err := browser.Close(ctx); err != nil {
		log.Fatalf("close browser session: %v", err)
	}
	fmt.Printf("[%s] session closed\n", browser.SessionID())
}

func waitForEnter(prompt string) {
	fmt.Println(prompt)
	reader := bufio.NewReader(os.Stdin)
	if _, err := reader.ReadString('\n'); err != nil {
		log.Fatalf("read keyboard confirmation: %v", err)
	}
}

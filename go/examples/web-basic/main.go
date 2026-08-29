package main

import (
	"context"
	"fmt"
	"log"
	"os"
	"strings"
	"time"

	allwright "allwright.dev"
)

const (
	defaultWebURL             = "https://themoderninternet.vercel.app"
	defaultWebEntrySelector   = "xpath=//div[contains(@class,'card')][.//h2[normalize-space()='Form Inputs']]//button[normalize-space()='Visit page']"
	defaultWebHeadingSelector = "xpath=//h1[text()=\"Form Inputs\"]"
	defaultWebHeadingText     = "Form Inputs"
)

func main() {
	ctx, cancel := context.WithTimeout(context.Background(), 60*time.Second)
	defer cancel()

	if err := allwright.SetServerAddr(serverAddr()); err != nil {
		log.Fatalf("set server addr: %v", err)
	}
	defer func() {
		if err := allwright.Shutdown(); err != nil {
			log.Printf("shutdown allwright: %v", err)
		}
	}()

	browser, err := allwright.LaunchFirefox(ctx, allwright.LaunchOptions{
		BrowserBinary: os.Getenv("ALLWRIGHT_BROWSER_BINARY"),
	})
	if err != nil {
		log.Fatalf("launch firefox: %v", err)
	}
	defer func() {
		if err := browser.Close(ctx); err != nil {
			log.Printf("close browser: %v", err)
		}
	}()

	page := browser.Page()
	if _, err := page.Navigate(ctx, envOr("ALLWRIGHT_WEB_URL", defaultWebURL)); err != nil {
		log.Fatalf("navigate: %v", err)
	}
	if _, err := page.Click(ctx, envOr("ALLWRIGHT_WEB_ENTRY_SELECTOR", defaultWebEntrySelector)); err != nil {
		log.Fatalf("click entry: %v", err)
	}
	if _, err := page.WaitForSelector(ctx, envOr("ALLWRIGHT_WEB_HEADING_SELECTOR", defaultWebHeadingSelector), allwright.WaitForSelectorOptions{
		Visible: boolPtr(true),
		Timeout: 10 * time.Second,
	}); err != nil {
		log.Fatalf("wait for heading: %v", err)
	}
	heading, err := page.TextContent(ctx, envOr("ALLWRIGHT_WEB_HEADING_SELECTOR", defaultWebHeadingSelector))
	if err != nil {
		log.Fatalf("read h1: %v", err)
	}
	expected := envOr("ALLWRIGHT_WEB_HEADING_TEXT", defaultWebHeadingText)
	if !strings.Contains(heading.Text, expected) {
		log.Fatalf("expected heading to contain %q, got %q", expected, heading.Text)
	}
	fmt.Printf("[go-web-basic] heading=%q\n", heading.Text)
}

func serverAddr() string {
	return envOr("ALLWRIGHT_SERVER_ADDR", "127.0.0.1:50051")
}

func envOr(name string, fallback string) string {
	value := os.Getenv(name)
	if value == "" {
		return fallback
	}
	return value
}

func boolPtr(value bool) *bool {
	return &value
}

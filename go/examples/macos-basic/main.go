package main

import (
	"context"
	"fmt"
	"log"
	"os"
	"time"

	allwright "allwright.dev"
)

func main() {
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Minute)
	defer cancel()

	if err := allwright.SetServerAddr(envOr("ALLWRIGHT_SERVER_ADDR", "127.0.0.1:50051")); err != nil {
		log.Fatalf("set server addr: %v", err)
	}
	defer func() {
		if err := allwright.Shutdown(); err != nil {
			log.Printf("shutdown allwright: %v", err)
		}
	}()

	desktop, err := allwright.Desktop.Mac.Connect(ctx, allwright.DesktopMacConnectOptions{
		AgentEndpoint: os.Getenv("ALLWRIGHT_MAC_AGENT_ENDPOINT"),
		Timeout:       30_000,
	})
	if err != nil {
		log.Fatalf("connect macOS desktop: %v", err)
	}
	app, err := desktop.Launch(ctx, allwright.DesktopMacLaunchOptions{
		AppID:   envOr("ALLWRIGHT_MAC_APP_ID", "com.apple.calculator"),
		Timeout: 60_000,
	})
	if err != nil {
		log.Fatalf("launch macOS app: %v", err)
	}

	screenshot, err := app.Screenshot(ctx)
	if err != nil {
		log.Fatalf("capture macOS screenshot: %v", err)
	}
	if len(screenshot) == 0 {
		log.Fatal("macOS screenshot is empty")
	}
	fmt.Printf("[go-macos-basic] captured %d bytes\n", len(screenshot))
}

func envOr(name string, fallback string) string {
	if value := os.Getenv(name); value != "" {
		return value
	}
	return fallback
}

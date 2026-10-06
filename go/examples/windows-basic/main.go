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

	desktop, err := allwright.Desktop.Windows.Connect(ctx, allwright.DesktopWindowsConnectOptions{
		AgentEndpoint: os.Getenv("ALLWRIGHT_WINDOWS_AGENT_ENDPOINT"),
		Timeout:       30_000,
	})
	if err != nil {
		log.Fatalf("connect Windows desktop: %v", err)
	}
	app, err := desktop.Launch(ctx, allwright.DesktopWindowsLaunchOptions{
		AppID:   envOr("ALLWRIGHT_WINDOWS_APP_ID", "notepad.exe"),
		Timeout: 60_000,
	})
	if err != nil {
		log.Fatalf("launch Windows app: %v", err)
	}

	screenshot, err := app.Screenshot(ctx)
	if err != nil {
		log.Fatalf("capture Windows screenshot: %v", err)
	}
	if len(screenshot) == 0 {
		log.Fatal("Windows screenshot is empty")
	}
	fmt.Printf("[go-windows-basic] captured %d bytes\n", len(screenshot))
}

func envOr(name string, fallback string) string {
	if value := os.Getenv(name); value != "" {
		return value
	}
	return fallback
}

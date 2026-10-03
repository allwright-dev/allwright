package main

import (
	"context"
	"fmt"
	"log"
	"os"
	"time"

	allwright "allwright.dev"
)

const (
	defaultIOSAppPath = "https://allwright.dev/Flights-simulator.ipa"
	expectedAlert     = "No account found. Please sign up first."
)

func main() {
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Minute)
	defer cancel()

	if err := allwright.SetServerAddr(envOr("ALLWRIGHT_SERVER_ADDR", "127.0.0.1:50051")); err != nil {
		log.Fatalf("set server addr: %v", err)
	}
	defer func() {
		if err := allwright.Shutdown(); err != nil {
			log.Printf("shutdown allwright: %v", err)
		}
	}()

	device, err := allwright.Mobile.IOS.Connect(ctx, allwright.MobileIOSConnectOptions{
		Device:        os.Getenv("ALLWRIGHT_IOS_DEVICE"),
		AgentEndpoint: os.Getenv("ALLWRIGHT_IOS_AGENT_ENDPOINT"),
		Timeout:       60_000,
	})
	if err != nil {
		log.Fatalf("connect ios device: %v", err)
	}

	app, err := device.Launch(ctx, allwright.MobileIOSLaunchOptions{
		AppPath:          envOr("ALLWRIGHT_IOS_APP_PATH", defaultIOSAppPath),
		AppID:            os.Getenv("ALLWRIGHT_IOS_APP_ID"),
		StopBeforeLaunch: true,
		Timeout:          120_000,
	})
	if err != nil {
		log.Fatalf("launch ios app: %v", err)
	}

	exact := allwright.TextOptions{Exact: true}
	login := allwright.RoleOptions{Name: "Login", Exact: true}
	if err := app.GetByRole("button", login).Click(ctx); err != nil {
		log.Fatalf("open login: %v", err)
	}
	if err := app.GetByLabel("Email", exact).Fill(ctx, "allwright@example.com"); err != nil {
		log.Fatalf("fill email: %v", err)
	}
	if err := app.GetByLabel("Password", exact).Fill(ctx, "not-a-real-password"); err != nil {
		log.Fatalf("fill password: %v", err)
	}
	if err := app.GetByRole("button", allwright.RoleOptions{Name: "Submit", Exact: true}).Click(ctx); err != nil {
		log.Fatalf("submit login: %v", err)
	}

	alert, err := app.GetByText(expectedAlert, exact).TextContent(ctx)
	if err != nil {
		log.Fatalf("read alert: %v", err)
	}
	if alert != expectedAlert {
		log.Fatalf("unexpected alert: %q", alert)
	}
	fmt.Printf("[go-ios-basic] %s\n", alert)
}

func envOr(name string, fallback string) string {
	value := os.Getenv(name)
	if value == "" {
		return fallback
	}
	return value
}

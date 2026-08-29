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
	defaultAndroidDevice       = "emulator-5554"
	defaultAndroidAppID        = "com.example.airticket"
	defaultAndroidTapSelector  = "Id=com.example.airticket:id/bottom_nav_account"
	defaultAndroidFillSelector = "xpath=//*[@text=\"Email\"]"
	defaultAndroidFillValue    = "user@example.com"
)

func main() {
	ctx, cancel := context.WithTimeout(context.Background(), 90*time.Second)
	defer cancel()

	if err := allwright.SetServerAddr(serverAddr()); err != nil {
		log.Fatalf("set server addr: %v", err)
	}
	defer func() {
		if err := allwright.Shutdown(); err != nil {
			log.Printf("shutdown allwright: %v", err)
		}
	}()

	device, err := allwright.Mobile.Android.Connect(ctx, allwright.MobileAndroidConnectOptions{
		Device:      envOr("ALLWRIGHT_ANDROID_DEVICE", defaultAndroidDevice),
		AdbEndpoint: os.Getenv("ALLWRIGHT_ANDROID_ADB_ENDPOINT"),
		Timeout:     30_000,
	})
	if err != nil {
		log.Fatalf("connect android device: %v", err)
	}

	app, err := device.Launch(ctx, launchOptionsFromEnv())
	if err != nil {
		log.Fatalf("launch android app: %v", err)
	}

	if _, err := app.Click(ctx, envOr("ALLWRIGHT_ANDROID_TAP_SELECTOR", defaultAndroidTapSelector)); err != nil {
		log.Fatalf("tap android control: %v", err)
	}
	if _, err := app.Fill(
		ctx,
		envOr("ALLWRIGHT_ANDROID_FILL_SELECTOR", defaultAndroidFillSelector),
		envOr("ALLWRIGHT_ANDROID_FILL_VALUE", defaultAndroidFillValue),
	); err != nil {
		log.Fatalf("fill android control: %v", err)
	}

	fmt.Printf("[go-android-basic] app_session_id=%s\n", app.SessionID())
}

func launchOptionsFromEnv() allwright.MobileAndroidLaunchOptions {
	apkPath := os.Getenv("ALLWRIGHT_ANDROID_APK_PATH")
	appID := envOr("ALLWRIGHT_ANDROID_APP_ID", defaultAndroidAppID)
	return allwright.MobileAndroidLaunchOptions{
		APKPath:        apkPath,
		AppID:          appID,
		LaunchActivity: os.Getenv("ALLWRIGHT_ANDROID_APP_ACTIVITY"),
		Timeout:        60_000,
	}
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

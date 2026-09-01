package allwright

import (
	"strings"
	"time"

	enginev1 "allwright.dev/gen/allwright/engine/v1"
)

func retryOptionsProto(timeout time.Duration) *enginev1.CommandRetryOptions {
	if timeout <= 0 {
		return nil
	}

	timeoutMS := timeout.Milliseconds()
	if timeoutMS <= 0 {
		timeoutMS = 1
	}

	return &enginev1.CommandRetryOptions{
		TimeoutMs:       optionalUint32(uint32(timeoutMS)),
		RetryIntervalMs: nil,
	}
}

func durationProto(value time.Duration) *uint32 {
	if value <= 0 {
		return nil
	}

	durationMS := value.Milliseconds()
	if durationMS <= 0 {
		durationMS = 1
	}

	return optionalUint32(uint32(durationMS))
}

func optionalUint32(value uint32) *uint32 {
	return &value
}

func optionalBool(value bool) *bool {
	return &value
}

func optionalString(value string) *string {
	trimmed := strings.TrimSpace(value)
	if trimmed == "" {
		return nil
	}
	return &trimmed
}

func firstCommandOptions(options []CommandOptions) CommandOptions {
	if len(options) > 0 {
		return options[0]
	}
	return CommandOptions{}
}

func firstScreenshotOptions(options []ScreenshotOptions) ScreenshotOptions {
	if len(options) > 0 {
		return options[0]
	}
	return ScreenshotOptions{}
}

func firstHighlightOptions(options []HighlightOptions) HighlightOptions {
	if len(options) > 0 {
		return options[0]
	}
	return HighlightOptions{}
}

func firstPressOptions(options []PressOptions) PressOptions {
	if len(options) > 0 {
		return options[0]
	}
	return PressOptions{}
}

func firstWaitForSelectorOptions(options []WaitForSelectorOptions) WaitForSelectorOptions {
	if len(options) > 0 {
		return options[0]
	}
	return WaitForSelectorOptions{}
}

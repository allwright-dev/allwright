package allwright

import (
	"context"
	"fmt"

	enginev1 "allwright.dev/gen/allwright/engine/v1"
)

func (t *AndroidApp) AccessibilitySnapshot(ctx context.Context, options ...AccessibilitySnapshotOptions) (string, error) {
	if t == nil {
		return "", fmt.Errorf("android app is nil")
	}
	resolved := AccessibilitySnapshotOptions{Format: "json"}
	if len(options) > 0 {
		resolved = options[0]
	}
	if resolved.Format == "" {
		resolved.Format = "json"
	}
	if resolved.Format != "json" && resolved.Format != "yaml" {
		return "", fmt.Errorf("accessibility snapshot format must be 'json' or 'yaml'")
	}
	if resolved.Mode == "" {
		resolved.Mode = "default"
	}
	switch resolved.Mode {
	case "default", "ai", "autoexpect", "codegen":
	default:
		return "", fmt.Errorf("invalid accessibility snapshot mode")
	}
	if err := t.ensureStream(ctx); err != nil {
		return "", err
	}
	if t.closed {
		return "", fmt.Errorf("android app session %s is closed", t.sessionID)
	}
	if err := t.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: t.surfaceSessionID,
		ContextSessionId: t.sessionID,
		Command: &enginev1.ContextSessionCommand_AccessibilitySnapshot{
			AccessibilitySnapshot: &enginev1.AccessibilitySnapshotCommand{
				Format:       resolved.Format,
				Mode:         resolved.Mode,
				RetryOptions: retryOptionsProto(resolved.Timeout),
			},
		},
	}); err != nil {
		return "", fmt.Errorf("send accessibility snapshot: %w", err)
	}
	for {
		event, err := t.stream.Recv()
		if err != nil {
			return "", fmt.Errorf("receive accessibility snapshot: %w", err)
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_Attached:
			t.attached = true
		case *enginev1.ContextSessionEvent_AccessibilitySnapshotCaptured:
			return payload.AccessibilitySnapshotCaptured.GetSnapshot(), nil
		case *enginev1.ContextSessionEvent_Error:
			return "", fmt.Errorf("accessibility snapshot failed: %s", payload.Error.GetMessage())
		case *enginev1.ContextSessionEvent_Closed:
			t.closed = true
			return "", fmt.Errorf("android app session %s closed while capturing accessibility snapshot", t.sessionID)
		}
	}
}

package allwright

import (
	"context"
	"fmt"

	enginev1 "allwright.dev/gen/allwright/engine/v1"
)

type HookType[T any] struct {
	name   string
	decode func(*Browser, *enginev1.HookCompletedEvent) (T, error)
}

type Hook[T any] struct {
	browser  *Browser
	id       string
	hookType HookType[T]
}

func (h *Hook[T]) ID() string {
	if h == nil {
		return ""
	}
	return h.id
}

var Hooks = struct {
	NewPage HookType[*Page]
}{
	NewPage: HookType[*Page]{
		name: "new_page",
		decode: func(browser *Browser, completed *enginev1.HookCompletedEvent) (*Page, error) {
			result, ok := completed.GetResult().(*enginev1.HookCompletedEvent_NewPage)
			if !ok || result.NewPage.GetContextSessionId() == "" {
				return nil, fmt.Errorf("new page hook completed with an invalid result")
			}
			return &Tab{
				runtime:          browser.runtime,
				browserSessionID: browser.sessionID,
				sessionID:        result.NewPage.GetContextSessionId(),
			}, nil
		},
	},
}

func RegisterHook[T any](ctx context.Context, browser *Browser, hookType HookType[T]) (*Hook[T], error) {
	if browser == nil {
		return nil, fmt.Errorf("browser is nil")
	}
	browser.mu.Lock()
	defer browser.mu.Unlock()
	if browser.closed {
		return nil, fmt.Errorf("browser session %s is closed", browser.sessionID)
	}
	if hookType.name != "new_page" {
		return nil, fmt.Errorf("unsupported hook type: %s", hookType.name)
	}
	if err := browser.stream.Send(&enginev1.SurfaceSessionCommand{
		Command: &enginev1.SurfaceSessionCommand_RegisterHook{
			RegisterHook: &enginev1.RegisterHookCommand{
				Hook: &enginev1.RegisterHookCommand_NewPage{NewPage: &enginev1.RegisterNewPageHook{}},
			},
		},
	}); err != nil {
		return nil, fmt.Errorf("send RegisterHookCommand: %w", err)
	}

	for {
		select {
		case <-ctx.Done():
			return nil, ctx.Err()
		default:
		}
		event, err := browser.stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive browser session event while registering hook: %w", err)
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.SurfaceSessionEvent_HookRegistered:
			return &Hook[T]{browser: browser, id: payload.HookRegistered.GetHookId(), hookType: hookType}, nil
		case *enginev1.SurfaceSessionEvent_Error:
			return nil, fmt.Errorf("browser session error while registering hook: %s", payload.Error.GetMessage())
		}
	}
}

func (h *Hook[T]) Wait(ctx context.Context, options ...CommandOptions) (T, error) {
	var zero T
	if h == nil || h.browser == nil {
		return zero, fmt.Errorf("hook is nil")
	}
	browser := h.browser
	browser.mu.Lock()
	defer browser.mu.Unlock()
	if browser.closed {
		return zero, fmt.Errorf("browser session %s is closed", browser.sessionID)
	}
	commandOptions := firstCommandOptions(options)
	if err := browser.stream.Send(&enginev1.SurfaceSessionCommand{
		Command: &enginev1.SurfaceSessionCommand_WaitForHook{
			WaitForHook: &enginev1.WaitForHookCommand{
				HookId:       h.id,
				RetryOptions: retryOptionsProto(commandOptions.Timeout),
			},
		},
	}); err != nil {
		return zero, fmt.Errorf("send WaitForHookCommand: %w", err)
	}

	for {
		select {
		case <-ctx.Done():
			return zero, ctx.Err()
		default:
		}
		event, err := browser.stream.Recv()
		if err != nil {
			return zero, fmt.Errorf("receive browser session event while waiting for hook: %w", err)
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.SurfaceSessionEvent_HookCompleted:
			if payload.HookCompleted.GetHookId() == h.id {
				return h.hookType.decode(browser, payload.HookCompleted)
			}
		case *enginev1.SurfaceSessionEvent_Error:
			return zero, fmt.Errorf("browser session error while waiting for hook: %s", payload.Error.GetMessage())
		}
	}
}

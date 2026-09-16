package allwright

import (
	"context"
	"fmt"

	enginev1 "allwright.dev/gen/allwright/engine/v1"
)

type HookType[T any] struct {
	name   string
	decode func(*Tab, *enginev1.HookCompletedEvent) (T, error)
}

type Hook[T any] struct {
	page     *Tab
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
		decode: func(page *Tab, completed *enginev1.HookCompletedEvent) (*Page, error) {
			result, ok := completed.GetResult().(*enginev1.HookCompletedEvent_NewPage)
			if !ok || result.NewPage.GetContextSessionId() == "" {
				return nil, fmt.Errorf("new page hook completed with an invalid result")
			}
			return &Tab{
				runtime:          page.runtime,
				browserSessionID: page.browserSessionID,
				sessionID:        result.NewPage.GetContextSessionId(),
			}, nil
		},
	},
}

func RegisterHook[T any](ctx context.Context, page *Page, hookType HookType[T]) (*Hook[T], error) {
	if page == nil {
		return nil, fmt.Errorf("page is nil")
	}
	page.mu.Lock()
	defer page.mu.Unlock()
	if err := page.ensureStream(ctx); err != nil {
		return nil, err
	}
	if page.closed {
		return nil, fmt.Errorf("page session %s is closed", page.sessionID)
	}
	if hookType.name != "new_page" {
		return nil, fmt.Errorf("unsupported hook type: %s", hookType.name)
	}
	if err := page.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: page.browserSessionID,
		ContextSessionId: page.sessionID,
		Command: &enginev1.ContextSessionCommand_RegisterHook{
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
		event, err := page.stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive page session event while registering hook: %w", err)
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_HookRegistered:
			return &Hook[T]{page: page, id: payload.HookRegistered.GetHookId(), hookType: hookType}, nil
		case *enginev1.ContextSessionEvent_Error:
			return nil, fmt.Errorf("page session error while registering hook: %s", payload.Error.GetMessage())
		}
	}
}

func (h *Hook[T]) Wait(ctx context.Context, options ...CommandOptions) (T, error) {
	var zero T
	if h == nil || h.page == nil {
		return zero, fmt.Errorf("hook is nil")
	}
	page := h.page
	page.mu.Lock()
	defer page.mu.Unlock()
	if err := page.ensureStream(ctx); err != nil {
		return zero, err
	}
	if page.closed {
		return zero, fmt.Errorf("page session %s is closed", page.sessionID)
	}
	commandOptions := firstCommandOptions(options)
	if err := page.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: page.browserSessionID,
		ContextSessionId: page.sessionID,
		Command: &enginev1.ContextSessionCommand_WaitForHook{
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
		event, err := page.stream.Recv()
		if err != nil {
			return zero, fmt.Errorf("receive page session event while waiting for hook: %w", err)
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_HookCompleted:
			if payload.HookCompleted.GetHookId() == h.id {
				return h.hookType.decode(page, payload.HookCompleted)
			}
		case *enginev1.ContextSessionEvent_Error:
			return zero, fmt.Errorf("page session error while waiting for hook: %s", payload.Error.GetMessage())
		}
	}
}

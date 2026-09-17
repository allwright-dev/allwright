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

type FileChooser struct {
	page       *Tab
	id         string
	isMultiple bool
}

type Download struct {
	page              *Tab
	id                string
	url               string
	suggestedFilename string
}

func (d *Download) ID() string { return d.id }

func (d *Download) Page() *Page { return d.page }

func (d *Download) URL() string { return d.url }

func (d *Download) SuggestedFilename() string { return d.suggestedFilename }

func (d *Download) SaveAs(ctx context.Context, path string, options ...CommandOptions) error {
	if d == nil || d.page == nil {
		return fmt.Errorf("download is nil")
	}
	page := d.page
	page.mu.Lock()
	defer page.mu.Unlock()
	if err := page.ensureStream(ctx); err != nil {
		return err
	}
	commandOptions := firstCommandOptions(options)
	if err := page.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: page.browserSessionID,
		ContextSessionId: page.sessionID,
		Command: &enginev1.ContextSessionCommand_SaveDownload{
			SaveDownload: &enginev1.SaveDownloadCommand{
				DownloadId:   d.id,
				Path:         path,
				RetryOptions: retryOptionsProto(commandOptions.Timeout),
			},
		},
	}); err != nil {
		return fmt.Errorf("send SaveDownloadCommand: %w", err)
	}
	for {
		event, err := page.stream.Recv()
		if err != nil {
			return fmt.Errorf("receive page session event while saving download: %w", err)
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_DownloadSaved:
			if payload.DownloadSaved.GetDownloadId() == d.id {
				return nil
			}
		case *enginev1.ContextSessionEvent_Error:
			return fmt.Errorf("page session error while saving download: %s", payload.Error.GetMessage())
		}
	}
}

func (f *FileChooser) ID() string { return f.id }

func (f *FileChooser) Page() *Page { return f.page }

func (f *FileChooser) IsMultiple() bool { return f.isMultiple }

func (f *FileChooser) SetFiles(ctx context.Context, files []string, options ...CommandOptions) error {
	if f == nil || f.page == nil {
		return fmt.Errorf("file chooser is nil")
	}
	if !f.isMultiple && len(files) > 1 {
		return fmt.Errorf("file chooser does not accept multiple files")
	}
	page := f.page
	page.mu.Lock()
	defer page.mu.Unlock()
	if err := page.ensureStream(ctx); err != nil {
		return err
	}
	commandOptions := firstCommandOptions(options)
	if err := page.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: page.browserSessionID,
		ContextSessionId: page.sessionID,
		Command: &enginev1.ContextSessionCommand_SetFileChooserFiles{
			SetFileChooserFiles: &enginev1.SetFileChooserFilesCommand{
				FileChooserId: f.id,
				Files:         files,
				RetryOptions:  retryOptionsProto(commandOptions.Timeout),
			},
		},
	}); err != nil {
		return fmt.Errorf("send SetFileChooserFilesCommand: %w", err)
	}
	for {
		event, err := page.stream.Recv()
		if err != nil {
			return fmt.Errorf("receive page session event while setting chooser files: %w", err)
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_FileChooserFilesSet:
			if payload.FileChooserFilesSet.GetFileChooserId() == f.id {
				return nil
			}
		case *enginev1.ContextSessionEvent_Error:
			return fmt.Errorf("page session error while setting chooser files: %s", payload.Error.GetMessage())
		}
	}
}

func (h *Hook[T]) ID() string {
	if h == nil {
		return ""
	}
	return h.id
}

var Hooks = struct {
	NewPage     HookType[*Page]
	FileChooser HookType[*FileChooser]
	Download    HookType[*Download]
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
	FileChooser: HookType[*FileChooser]{
		name: "file_chooser",
		decode: func(page *Tab, completed *enginev1.HookCompletedEvent) (*FileChooser, error) {
			result, ok := completed.GetResult().(*enginev1.HookCompletedEvent_FileChooser)
			if !ok || result.FileChooser.GetFileChooserId() == "" {
				return nil, fmt.Errorf("file chooser hook completed with an invalid result")
			}
			return &FileChooser{
				page: page, id: result.FileChooser.GetFileChooserId(),
				isMultiple: result.FileChooser.GetIsMultiple(),
			}, nil
		},
	},
	Download: HookType[*Download]{
		name: "download",
		decode: func(page *Tab, completed *enginev1.HookCompletedEvent) (*Download, error) {
			result, ok := completed.GetResult().(*enginev1.HookCompletedEvent_Download)
			if !ok || result.Download.GetDownloadId() == "" {
				return nil, fmt.Errorf("download hook completed with an invalid result")
			}
			return &Download{
				page: page, id: result.Download.GetDownloadId(), url: result.Download.GetUrl(),
				suggestedFilename: result.Download.GetSuggestedFilename(),
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
	if hookType.name != "new_page" && hookType.name != "file_chooser" && hookType.name != "download" {
		return nil, fmt.Errorf("unsupported hook type: %s", hookType.name)
	}
	registerHook := &enginev1.RegisterHookCommand{}
	if hookType.name == "new_page" {
		registerHook.Hook = &enginev1.RegisterHookCommand_NewPage{NewPage: &enginev1.RegisterNewPageHook{}}
	} else if hookType.name == "file_chooser" {
		registerHook.Hook = &enginev1.RegisterHookCommand_FileChooser{FileChooser: &enginev1.RegisterFileChooserHook{}}
	} else {
		registerHook.Hook = &enginev1.RegisterHookCommand_Download{Download: &enginev1.RegisterDownloadHook{}}
	}
	if err := page.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: page.browserSessionID,
		ContextSessionId: page.sessionID,
		Command: &enginev1.ContextSessionCommand_RegisterHook{
			RegisterHook: registerHook,
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

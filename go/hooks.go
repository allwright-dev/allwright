package allwright

import (
	"context"
	"crypto/rand"
	"fmt"
	"io"
	"os"
	"path/filepath"

	enginev1 "allwright.dev/gen/allwright/engine/v1"
)

type HookType[T any] struct {
	name   string
	decode func(any, *enginev1.HookCompletedEvent) (T, error)
}

// HookOwner is a web page or Android app that can register typed hooks.
type HookOwner interface {
	hookOwner()
}

func (*Tab) hookOwner()        {}
func (*AndroidApp) hookOwner() {}

type Hook[T any] struct {
	page     *Tab
	waitFn   func(context.Context, ...CommandOptions) (T, error)
	id       string
	hookType HookType[T]
}

type FileChooser struct {
	page       *Tab
	app        *AndroidApp
	id         string
	isMultiple bool
}

type Download struct {
	page              *Tab
	app               *AndroidApp
	id                string
	url               string
	suggestedFilename string
}

func (d *Download) ID() string { return d.id }

func (d *Download) Page() *Page { return d.page }

func (d *Download) App() *AndroidApp { return d.app }

func (d *Download) URL() string { return d.url }

func (d *Download) SuggestedFilename() string { return d.suggestedFilename }

func (d *Download) SaveAs(ctx context.Context, path string, options ...CommandOptions) error {
	if d == nil || (d.page == nil && d.app == nil) {
		return fmt.Errorf("download is nil")
	}
	if d.app != nil {
		return d.app.saveHookDownload(ctx, d.id, path, options...)
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
				return downloadFileToClient(ctx, page, payload.DownloadSaved.GetFileId(), path)
			}
		case *enginev1.ContextSessionEvent_Error:
			return fmt.Errorf("page session error while saving download: %s", payload.Error.GetMessage())
		}
	}
}

func (f *FileChooser) ID() string { return f.id }

func (f *FileChooser) Page() *Page { return f.page }

func (f *FileChooser) App() *AndroidApp { return f.app }

func (f *FileChooser) IsMultiple() bool { return f.isMultiple }

func (f *FileChooser) SetFiles(ctx context.Context, files []string, options ...CommandOptions) error {
	if f == nil || (f.page == nil && f.app == nil) {
		return fmt.Errorf("file chooser is nil")
	}
	if !f.isMultiple && len(files) > 1 {
		return fmt.Errorf("file chooser does not accept multiple files")
	}
	if f.app != nil {
		return f.app.setHookFileChooserFiles(ctx, f.id, files, options...)
	}
	page := f.page
	page.mu.Lock()
	defer page.mu.Unlock()
	if err := page.ensureStream(ctx); err != nil {
		return err
	}
	commandOptions := firstCommandOptions(options)
	fileIDs := make([]string, 0, len(files))
	for _, path := range files {
		fileID, err := uploadClientFile(ctx, page, path)
		if err != nil {
			return err
		}
		fileIDs = append(fileIDs, fileID)
	}
	if err := page.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: page.browserSessionID,
		ContextSessionId: page.sessionID,
		Command: &enginev1.ContextSessionCommand_SetFileChooserFiles{
			SetFileChooserFiles: &enginev1.SetFileChooserFilesCommand{
				FileChooserId: f.id,
				FileIds:       fileIDs,
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

func transferID() (string, error) {
	var value [16]byte
	if _, err := rand.Read(value[:]); err != nil {
		return "", fmt.Errorf("create file transfer id: %w", err)
	}
	return fmt.Sprintf("%x", value[:]), nil
}

func uploadClientFile(ctx context.Context, page *Tab, path string) (string, error) {
	source, err := os.Open(path)
	if err != nil {
		return "", fmt.Errorf("open upload %s: %w", path, err)
	}
	defer source.Close()
	info, err := source.Stat()
	if err != nil {
		return "", fmt.Errorf("inspect upload %s: %w", path, err)
	}
	id, err := transferID()
	if err != nil {
		return "", err
	}
	var offset uint64
	for {
		buffer := make([]byte, 256*1024)
		count, readErr := source.Read(buffer)
		last := offset+uint64(count) >= uint64(info.Size())
		if readErr != nil && readErr != io.EOF {
			return "", fmt.Errorf("read upload %s: %w", path, readErr)
		}
		if err := page.stream.Send(&enginev1.ContextSessionCommand{
			SurfaceSessionId: page.browserSessionID,
			ContextSessionId: page.sessionID,
			Command: &enginev1.ContextSessionCommand_UploadFileChunk{
				UploadFileChunk: &enginev1.UploadFileChunkCommand{
					TransferId: id,
					Name:       filepath.Base(path),
					Offset:     offset,
					Data:       buffer[:count],
					Last:       last,
				},
			},
		}); err != nil {
			return "", fmt.Errorf("send UploadFileChunkCommand: %w", err)
		}
		offset += uint64(count)
		if last {
			break
		}
	}
	for {
		event, err := page.stream.Recv()
		if err != nil {
			return "", fmt.Errorf("receive file upload event: %w", err)
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_FileUploaded:
			if payload.FileUploaded.GetTransferId() == id {
				return payload.FileUploaded.GetFileId(), nil
			}
		case *enginev1.ContextSessionEvent_Error:
			return "", fmt.Errorf("page session error while uploading file: %s", payload.Error.GetMessage())
		}
	}
}

func downloadFileToClient(ctx context.Context, page *Tab, fileID, path string) error {
	id, err := transferID()
	if err != nil {
		return err
	}
	temporary := path + ".allwright-" + id + ".tmp"
	destination, err := os.OpenFile(temporary, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0o600)
	if err != nil {
		return fmt.Errorf("create download destination: %w", err)
	}
	complete := false
	defer func() {
		destination.Close()
		if !complete {
			_ = os.Remove(temporary)
		}
	}()
	var offset uint64
	for {
		if err := page.stream.Send(&enginev1.ContextSessionCommand{
			SurfaceSessionId: page.browserSessionID,
			ContextSessionId: page.sessionID,
			Command: &enginev1.ContextSessionCommand_ReadFileChunk{
				ReadFileChunk: &enginev1.ReadFileChunkCommand{FileId: fileID, Offset: offset, MaxBytes: 256 * 1024},
			},
		}); err != nil {
			return fmt.Errorf("send ReadFileChunkCommand: %w", err)
		}
		event, err := page.stream.Recv()
		if err != nil {
			return fmt.Errorf("receive file chunk: %w", err)
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_FileChunk:
			chunk := payload.FileChunk
			if chunk.GetFileId() != fileID || chunk.GetOffset() != offset {
				continue
			}
			if _, err := destination.Write(chunk.GetData()); err != nil {
				return fmt.Errorf("write download destination: %w", err)
			}
			offset += uint64(len(chunk.GetData()))
			if chunk.GetLast() {
				if err := destination.Close(); err != nil {
					return err
				}
				if err := os.Rename(temporary, path); err != nil {
					return fmt.Errorf("finish download destination: %w", err)
				}
				complete = true
				return nil
			}
		case *enginev1.ContextSessionEvent_Error:
			return fmt.Errorf("page session error while downloading file: %s", payload.Error.GetMessage())
		}
	}
}

func (app *AndroidApp) setHookFileChooserFiles(ctx context.Context, chooserID string, files []string, options ...CommandOptions) error {
	if err := app.ensureStream(ctx); err != nil {
		return err
	}
	fileIDs := make([]string, 0, len(files))
	for _, path := range files {
		id, err := uploadAndroidClientFile(app, path)
		if err != nil {
			return err
		}
		fileIDs = append(fileIDs, id)
	}
	commandOptions := firstCommandOptions(options)
	if err := app.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: app.surfaceSessionID, ContextSessionId: app.sessionID,
		Command: &enginev1.ContextSessionCommand_SetMobileFileChooserFiles{
			SetMobileFileChooserFiles: &enginev1.SetMobileFileChooserFilesCommand{
				FileChooserId: chooserID, FileIds: fileIDs, RetryOptions: retryOptionsProto(commandOptions.Timeout),
			},
		},
	}); err != nil {
		return err
	}
	for {
		event, err := app.stream.Recv()
		if err != nil {
			return err
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_MobileFileChooserFilesSet:
			if payload.MobileFileChooserFilesSet.GetFileChooserId() == chooserID {
				return nil
			}
		case *enginev1.ContextSessionEvent_Error:
			return fmt.Errorf("Android file chooser failed: %s", payload.Error.GetMessage())
		}
	}
}

func (app *AndroidApp) saveHookDownload(ctx context.Context, downloadID, path string, options ...CommandOptions) error {
	if err := app.ensureStream(ctx); err != nil {
		return err
	}
	commandOptions := firstCommandOptions(options)
	if err := app.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: app.surfaceSessionID, ContextSessionId: app.sessionID,
		Command: &enginev1.ContextSessionCommand_SaveMobileDownload{SaveMobileDownload: &enginev1.SaveMobileDownloadCommand{
			DownloadId: downloadID, RetryOptions: retryOptionsProto(commandOptions.Timeout),
		}},
	}); err != nil {
		return err
	}
	for {
		event, err := app.stream.Recv()
		if err != nil {
			return err
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_MobileDownloadSaved:
			if payload.MobileDownloadSaved.GetDownloadId() == downloadID {
				return downloadAndroidFileToClient(app, payload.MobileDownloadSaved.GetFileId(), path)
			}
		case *enginev1.ContextSessionEvent_Error:
			return fmt.Errorf("Android download failed: %s", payload.Error.GetMessage())
		}
	}
}

func uploadAndroidClientFile(app *AndroidApp, path string) (string, error) {
	source, err := os.Open(path)
	if err != nil {
		return "", err
	}
	defer source.Close()
	info, err := source.Stat()
	if err != nil {
		return "", err
	}
	id, err := transferID()
	if err != nil {
		return "", err
	}
	var offset uint64
	for {
		buffer := make([]byte, 256*1024)
		count, readErr := source.Read(buffer)
		if readErr != nil && readErr != io.EOF {
			return "", readErr
		}
		last := offset+uint64(count) >= uint64(info.Size())
		if err := app.stream.Send(&enginev1.ContextSessionCommand{
			SurfaceSessionId: app.surfaceSessionID, ContextSessionId: app.sessionID,
			Command: &enginev1.ContextSessionCommand_UploadFileChunk{UploadFileChunk: &enginev1.UploadFileChunkCommand{
				TransferId: id, Name: filepath.Base(path), Offset: offset, Data: buffer[:count], Last: last,
			}},
		}); err != nil {
			return "", err
		}
		offset += uint64(count)
		if last {
			break
		}
	}
	for {
		event, err := app.stream.Recv()
		if err != nil {
			return "", err
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_FileUploaded:
			if payload.FileUploaded.GetTransferId() == id {
				return payload.FileUploaded.GetFileId(), nil
			}
		case *enginev1.ContextSessionEvent_Error:
			return "", fmt.Errorf("Android file upload failed: %s", payload.Error.GetMessage())
		}
	}
}

func downloadAndroidFileToClient(app *AndroidApp, fileID, path string) error {
	id, err := transferID()
	if err != nil {
		return err
	}
	temporary := path + ".allwright-" + id + ".tmp"
	destination, err := os.OpenFile(temporary, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0o600)
	if err != nil {
		return err
	}
	complete := false
	defer func() {
		destination.Close()
		if !complete {
			_ = os.Remove(temporary)
		}
	}()
	var offset uint64
	for {
		if err := app.stream.Send(&enginev1.ContextSessionCommand{
			SurfaceSessionId: app.surfaceSessionID, ContextSessionId: app.sessionID,
			Command: &enginev1.ContextSessionCommand_ReadFileChunk{ReadFileChunk: &enginev1.ReadFileChunkCommand{
				FileId: fileID, Offset: offset, MaxBytes: 256 * 1024,
			}},
		}); err != nil {
			return err
		}
		event, err := app.stream.Recv()
		if err != nil {
			return err
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_FileChunk:
			chunk := payload.FileChunk
			if chunk.GetFileId() != fileID || chunk.GetOffset() != offset {
				continue
			}
			if _, err := destination.Write(chunk.GetData()); err != nil {
				return err
			}
			offset += uint64(len(chunk.GetData()))
			if chunk.GetLast() {
				if err := destination.Close(); err != nil {
					return err
				}
				if err := os.Rename(temporary, path); err != nil {
					return err
				}
				complete = true
				return nil
			}
		case *enginev1.ContextSessionEvent_Error:
			return fmt.Errorf("Android file download failed: %s", payload.Error.GetMessage())
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
	Dialog      HookType[*Dialog]
	NewPage     HookType[*Page]
	FileChooser HookType[*FileChooser]
	Download    HookType[*Download]
}{
	Dialog: HookType[*Dialog]{name: "dialog", decode: func(owner any, completed *enginev1.HookCompletedEvent) (*Dialog, error) {
		page, ok := owner.(*Page)
		d := completed.GetDialog()
		if !ok || d == nil || d.GetDialogId() == "" {
			return nil, fmt.Errorf("dialog hook completed with an invalid result")
		}
		return &Dialog{page: page, id: d.GetDialogId(), kind: d.GetType(), message: d.GetMessage(), defaultValue: d.GetDefaultValue()}, nil
	}},
	NewPage: HookType[*Page]{
		name: "new_page",
		decode: func(context any, completed *enginev1.HookCompletedEvent) (*Page, error) {
			page, ok := context.(*Tab)
			if !ok {
				return nil, fmt.Errorf("new page hook requires a web page")
			}
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
		decode: func(context any, completed *enginev1.HookCompletedEvent) (*FileChooser, error) {
			switch result := completed.GetResult().(type) {
			case *enginev1.HookCompletedEvent_FileChooser:
				page, ok := context.(*Tab)
				if !ok || result.FileChooser.GetFileChooserId() == "" {
					break
				}
				return &FileChooser{page: page, id: result.FileChooser.GetFileChooserId(), isMultiple: result.FileChooser.GetIsMultiple()}, nil
			case *enginev1.HookCompletedEvent_MobileFileChooser:
				app, ok := context.(*AndroidApp)
				if !ok || result.MobileFileChooser.GetFileChooserId() == "" {
					break
				}
				return &FileChooser{app: app, id: result.MobileFileChooser.GetFileChooserId(), isMultiple: result.MobileFileChooser.GetIsMultiple()}, nil
			}
			return nil, fmt.Errorf("file chooser hook completed with an invalid result")
		},
	},
	Download: HookType[*Download]{
		name: "download",
		decode: func(context any, completed *enginev1.HookCompletedEvent) (*Download, error) {
			switch result := completed.GetResult().(type) {
			case *enginev1.HookCompletedEvent_Download:
				page, ok := context.(*Tab)
				if !ok || result.Download.GetDownloadId() == "" {
					break
				}
				return &Download{page: page, id: result.Download.GetDownloadId(), url: result.Download.GetUrl(), suggestedFilename: result.Download.GetSuggestedFilename()}, nil
			case *enginev1.HookCompletedEvent_MobileDownload:
				app, ok := context.(*AndroidApp)
				if !ok || result.MobileDownload.GetDownloadId() == "" {
					break
				}
				return &Download{app: app, id: result.MobileDownload.GetDownloadId(), suggestedFilename: result.MobileDownload.GetSuggestedFilename()}, nil
			}
			return nil, fmt.Errorf("download hook completed with an invalid result")
		},
	},
}

func RegisterHook[T any](ctx context.Context, owner HookOwner, hookType HookType[T]) (*Hook[T], error) {
	if app, ok := owner.(*AndroidApp); ok {
		return registerAndroidHook(ctx, app, hookType)
	}
	page, ok := owner.(*Page)
	if !ok {
		return nil, fmt.Errorf("hook owner must be a web page or Android app")
	}
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
	if hookType.name != "dialog" && hookType.name != "new_page" && hookType.name != "file_chooser" && hookType.name != "download" {
		return nil, fmt.Errorf("unsupported hook type: %s", hookType.name)
	}
	registerHook := &enginev1.RegisterHookCommand{}
	if hookType.name == "dialog" {
		registerHook.Hook = &enginev1.RegisterHookCommand_Dialog{Dialog: &enginev1.RegisterDialogHook{}}
	} else if hookType.name == "new_page" {
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
			hook := &Hook[T]{page: page, id: payload.HookRegistered.GetHookId(), hookType: hookType}
			hook.waitFn = func(waitCtx context.Context, waitOptions ...CommandOptions) (T, error) {
				return waitPageHook(waitCtx, page, hook.id, hookType, waitOptions...)
			}
			return hook, nil
		case *enginev1.ContextSessionEvent_Error:
			return nil, fmt.Errorf("page session error while registering hook: %s", payload.Error.GetMessage())
		}
	}
}

func registerAndroidHook[T any](ctx context.Context, app *AndroidApp, hookType HookType[T]) (*Hook[T], error) {
	if app == nil {
		return nil, fmt.Errorf("android app is nil")
	}
	if err := app.ensureStream(ctx); err != nil {
		return nil, err
	}
	register := &enginev1.RegisterHookCommand{}
	switch hookType.name {
	case "file_chooser":
		register.Hook = &enginev1.RegisterHookCommand_MobileFileChooser{MobileFileChooser: &enginev1.RegisterMobileFileChooserHook{}}
	case "download":
		register.Hook = &enginev1.RegisterHookCommand_MobileDownload{MobileDownload: &enginev1.RegisterMobileDownloadHook{}}
	default:
		return nil, fmt.Errorf("hook type %s is not supported by Android apps", hookType.name)
	}
	if err := app.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: app.surfaceSessionID, ContextSessionId: app.sessionID,
		Command: &enginev1.ContextSessionCommand_RegisterHook{RegisterHook: register},
	}); err != nil {
		return nil, err
	}
	for {
		event, err := app.stream.Recv()
		if err != nil {
			return nil, err
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_HookRegistered:
			hook := &Hook[T]{id: payload.HookRegistered.GetHookId(), hookType: hookType}
			hook.waitFn = func(waitCtx context.Context, waitOptions ...CommandOptions) (T, error) {
				return waitAndroidHook(waitCtx, app, hook.id, hookType, waitOptions...)
			}
			return hook, nil
		case *enginev1.ContextSessionEvent_Error:
			return nil, fmt.Errorf("Android hook registration failed: %s", payload.Error.GetMessage())
		}
	}
}

func waitAndroidHook[T any](ctx context.Context, app *AndroidApp, hookID string, hookType HookType[T], options ...CommandOptions) (T, error) {
	var zero T
	if err := app.ensureStream(ctx); err != nil {
		return zero, err
	}
	commandOptions := firstCommandOptions(options)
	if err := app.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: app.surfaceSessionID, ContextSessionId: app.sessionID,
		Command: &enginev1.ContextSessionCommand_WaitForHook{WaitForHook: &enginev1.WaitForHookCommand{
			HookId: hookID, RetryOptions: retryOptionsProto(commandOptions.Timeout),
		}},
	}); err != nil {
		return zero, err
	}
	for {
		event, err := app.stream.Recv()
		if err != nil {
			return zero, err
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_HookCompleted:
			if payload.HookCompleted.GetHookId() == hookID {
				return hookType.decode(app, payload.HookCompleted)
			}
		case *enginev1.ContextSessionEvent_Error:
			return zero, fmt.Errorf("Android hook wait failed: %s", payload.Error.GetMessage())
		}
	}
}

func (h *Hook[T]) Wait(ctx context.Context, options ...CommandOptions) (T, error) {
	var zero T
	if h == nil || h.waitFn == nil {
		return zero, fmt.Errorf("hook is nil")
	}
	return h.waitFn(ctx, options...)
}

func waitPageHook[T any](ctx context.Context, page *Tab, hookID string, hookType HookType[T], options ...CommandOptions) (T, error) {
	var zero T
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
				HookId:       hookID,
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
			if payload.HookCompleted.GetHookId() == hookID {
				return hookType.decode(page, payload.HookCompleted)
			}
		case *enginev1.ContextSessionEvent_Error:
			return zero, fmt.Errorf("page session error while waiting for hook: %s", payload.Error.GetMessage())
		}
	}
}

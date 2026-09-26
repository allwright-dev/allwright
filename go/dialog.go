package allwright

import (
	enginev1 "allwright.dev/gen/allwright/engine/v1"
	"context"
	"fmt"
)

// Dialog is a pending JavaScript alert, confirm, or prompt.
type Dialog struct {
	page                            *Page
	id, kind, message, defaultValue string
}

func (d *Dialog) ID() string           { return d.id }
func (d *Dialog) Page() *Page          { return d.page }
func (d *Dialog) Type() string         { return d.kind }
func (d *Dialog) Message() string      { return d.message }
func (d *Dialog) DefaultValue() string { return d.defaultValue }

// Accept optionally supplies prompt text; an explicit empty string clears the default.
func (d *Dialog) Accept(ctx context.Context, promptText ...string) error {
	if len(promptText) > 1 {
		return fmt.Errorf("accept expects at most one prompt text")
	}
	var text *string
	if len(promptText) == 1 {
		text = &promptText[0]
	}
	return d.AcceptWithOptions(ctx, text, CommandOptions{})
}

// AcceptWithOptions accepts once within the command timeout. Nil text keeps the default.
func (d *Dialog) AcceptWithOptions(ctx context.Context, text *string, options CommandOptions) error {
	return d.handle(ctx, true, text, options)
}
func (d *Dialog) Dismiss(ctx context.Context, options ...CommandOptions) error {
	return d.handle(ctx, false, nil, firstCommandOptions(options))
}
func (d *Dialog) handle(ctx context.Context, accept bool, text *string, options CommandOptions) error {
	if d == nil || d.page == nil {
		return fmt.Errorf("dialog is nil")
	}
	p := d.page
	p.mu.Lock()
	defer p.mu.Unlock()
	if p.closed {
		return fmt.Errorf("page is closed")
	}
	if err := p.ensureStream(ctx); err != nil {
		return err
	}
	if err := p.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: p.browserSessionID, ContextSessionId: p.sessionID,
		Command: &enginev1.ContextSessionCommand_HandleDialog{HandleDialog: &enginev1.HandleDialogCommand{DialogId: d.id, Accept: accept, PromptText: text, RetryOptions: retryOptionsProto(options.Timeout)}},
	}); err != nil {
		return err
	}
	for {
		event, err := p.stream.Recv()
		if err != nil {
			return err
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_DialogHandled:
			if payload.DialogHandled.GetDialogId() == d.id {
				return nil
			}
		case *enginev1.ContextSessionEvent_Error:
			return fmt.Errorf("handle dialog: %s", payload.Error.GetMessage())
		case *enginev1.ContextSessionEvent_Closed:
			p.closed = true
			return fmt.Errorf("page closed while handling dialog")
		}
	}
}

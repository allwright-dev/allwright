package allwright

import (
	enginev1 "allwright.dev/gen/allwright/engine/v1"
	"testing"
)

func TestDialogHookDecodesTypedResult(t *testing.T) {
	page := &Page{}
	result := &enginev1.HookCompletedEvent{Result: &enginev1.HookCompletedEvent_Dialog{Dialog: &enginev1.DialogHookResult{
		DialogId: "dialog", Type: "prompt", Message: "Name?", DefaultValue: "Ada",
	}}}
	dialog, err := Hooks.Dialog.decode(page, result)
	if err != nil {
		t.Fatal(err)
	}
	if dialog.Page() != page || dialog.ID() != "dialog" || dialog.Type() != "prompt" || dialog.Message() != "Name?" || dialog.DefaultValue() != "Ada" {
		t.Fatalf("unexpected dialog: %#v", dialog)
	}
	if _, err := Hooks.Dialog.decode(page, &enginev1.HookCompletedEvent{}); err == nil {
		t.Fatal("expected invalid result error")
	}
	if _, err := Hooks.Dialog.decode(&AndroidApp{}, result); err == nil {
		t.Fatal("expected unsupported owner error")
	}
}

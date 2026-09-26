package dev.allwright.client;

import static org.junit.jupiter.api.Assertions.*;
import dev.allwright.engine.v1.DialogHookResult;
import dev.allwright.engine.v1.HookCompletedEvent;
import org.junit.jupiter.api.Test;

class DialogTest {
    @Test void decodesDialogMetadataAndRejectsInvalidResults() {
        var event = HookCompletedEvent.newBuilder().setDialog(DialogHookResult.newBuilder()
                .setDialogId("dialog").setType("prompt").setMessage("Name?").setDefaultValue("Ada")).build();
        var dialog = Hooks.DIALOG.decode(null, event);
        assertEquals("dialog", dialog.id());
        assertEquals("prompt", dialog.type());
        assertEquals("Name?", dialog.message());
        assertEquals("Ada", dialog.defaultValue());
        assertThrows(AllwrightException.class, () -> Hooks.DIALOG.decode(null, HookCompletedEvent.getDefaultInstance()));
    }
}

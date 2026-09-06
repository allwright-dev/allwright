package dev.allwright.client;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.regex.Pattern;

final class WebSelectorSupport {
    private WebSelectorSupport() {}
    static String json(Object value) {
        if (value instanceof String text) return SelectorSupport.quoteJson(text);
        if (value instanceof Boolean || value instanceof Integer) return value.toString();
        if (value instanceof Pattern pattern) {
            int supported = Pattern.CASE_INSENSITIVE | Pattern.MULTILINE | Pattern.DOTALL | Pattern.UNICODE_CASE;
            if ((pattern.flags() & ~supported) != 0) throw new IllegalArgumentException("Unsupported locator regex flags");
            String flags = ((pattern.flags() & Pattern.CASE_INSENSITIVE) != 0 ? "i" : "")
                    + ((pattern.flags() & Pattern.MULTILINE) != 0 ? "m" : "")
                    + ((pattern.flags() & Pattern.DOTALL) != 0 ? "s" : "");
            return json(Map.of("regex", pattern.pattern(), "flags", flags));
        }
        if (value instanceof Map<?, ?> map) {
            return "{" + map.entrySet().stream().filter(e -> e.getValue() != null)
                    .map(e -> json(e.getKey()) + ":" + json(e.getValue()))
                    .collect(java.util.stream.Collectors.joining(",")) + "}";
        }
        throw new IllegalArgumentException("Text matcher must be a string or Pattern");
    }
    static String selector(Map<String, Object> spec) { return "aw=" + SelectorSupport.quoteJson(json(spec)); }
    static String text(String kind, Object text, TextOptions options) { return selector(Map.of("kind", kind, "text", text, "exact", options.exact())); }
    static String role(String role, RoleOptions options) {
        if (options.level != null && options.level < 1) throw new IllegalArgumentException("Role level must be positive");
        Map<String, Object> spec = new LinkedHashMap<>();
        spec.put("kind", "role"); spec.put("role", role);
        spec.put("name", options.name);
        spec.put("exact", options.exact);
        spec.put("checked", options.checked);
        spec.put("disabled", options.disabled);
        spec.put("expanded", options.expanded);
        spec.put("includeHidden", options.includeHidden);
        spec.put("level", options.level);
        spec.put("pressed", options.pressed);
        spec.put("selected", options.selected);
        return selector(spec);
    }
}

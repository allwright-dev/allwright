package dev.allwright.client;

final class SelectorSupport {
    private SelectorSupport() {}

    static String normalizeSelectorForTransport(String selector) {
        SelectorTransport parsed = parseSelectorForTransport(selector);
        return parsed.kind() + "=" + quoteJson(parsed.body());
    }

    static String chainSelectorForTransport(String parent, String child) {
        String normalizedParent = parent == null || parent.trim().isEmpty()
                ? ""
                : normalizeSelectorForTransport(parent);
        String normalizedChild = child == null || child.trim().isEmpty()
                ? ""
                : normalizeSelectorForTransport(child);
        if (normalizedParent.isEmpty()) {
            return normalizedChild;
        }
        if (normalizedChild.isEmpty()) {
            return normalizedParent;
        }
        return normalizedParent + " " + normalizedChild;
    }

    private static SelectorTransport parseSelectorForTransport(String selector) {
        String trimmed = selector == null ? "" : selector.trim();
        String lowered = trimmed.toLowerCase();
        if (lowered.startsWith("xpath=") || lowered.startsWith("xpath:")) {
            return new SelectorTransport("xpath", decodeSelectorBody(trimmed.substring(6)));
        }
        if (lowered.startsWith("css=") || lowered.startsWith("css:")) {
            return new SelectorTransport("css", decodeSelectorBody(trimmed.substring(4)));
        }
        if (
                trimmed.startsWith("//")
                        || trimmed.startsWith(".//")
                        || trimmed.startsWith("../")
                        || trimmed.startsWith("/")
                        || trimmed.startsWith("(")
        ) {
            return new SelectorTransport("xpath", trimmed);
        }
        return new SelectorTransport("css", trimmed);
    }

    private static String decodeSelectorBody(String body) {
        String candidate = body == null ? "" : body.trim();
        if (candidate.length() >= 2 && candidate.charAt(0) == '"' && candidate.charAt(candidate.length() - 1) == '"') {
            try {
                return unescapeJsonString(candidate.substring(1, candidate.length() - 1));
            } catch (IllegalArgumentException ignored) {
                return candidate;
            }
        }
        return candidate;
    }

    private static String quoteJson(String value) {
        StringBuilder builder = new StringBuilder(value.length() + 2);
        builder.append('"');
        for (int index = 0; index < value.length(); index++) {
            char ch = value.charAt(index);
            switch (ch) {
                case '"' -> builder.append("\\\"");
                case '\\' -> builder.append("\\\\");
                case '\b' -> builder.append("\\b");
                case '\f' -> builder.append("\\f");
                case '\n' -> builder.append("\\n");
                case '\r' -> builder.append("\\r");
                case '\t' -> builder.append("\\t");
                default -> {
                    if (ch < 0x20) {
                        builder.append(String.format("\\u%04x", (int) ch));
                    } else {
                        builder.append(ch);
                    }
                }
            }
        }
        builder.append('"');
        return builder.toString();
    }

    private static String unescapeJsonString(String value) {
        StringBuilder builder = new StringBuilder(value.length());
        for (int index = 0; index < value.length(); index++) {
            char ch = value.charAt(index);
            if (ch != '\\') {
                builder.append(ch);
                continue;
            }
            if (index + 1 >= value.length()) {
                throw new IllegalArgumentException("unterminated escape");
            }
            char escaped = value.charAt(++index);
            switch (escaped) {
                case '"', '\\', '/' -> builder.append(escaped);
                case 'b' -> builder.append('\b');
                case 'f' -> builder.append('\f');
                case 'n' -> builder.append('\n');
                case 'r' -> builder.append('\r');
                case 't' -> builder.append('\t');
                case 'u' -> {
                    if (index + 4 >= value.length()) {
                        throw new IllegalArgumentException("invalid unicode escape");
                    }
                    String hex = value.substring(index + 1, index + 5);
                    builder.append((char) Integer.parseInt(hex, 16));
                    index += 4;
                }
                default -> throw new IllegalArgumentException("unsupported escape");
            }
        }
        return builder.toString();
    }

    private record SelectorTransport(String kind, String body) {}
}

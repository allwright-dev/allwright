package dev.allwright.client;

import java.util.Iterator;
import java.util.Map;

final class MobileJsonSupport {
    private MobileJsonSupport() {}

    static String toJson(Object value) {
        if (value == null) {
            return "null";
        }
        if (value instanceof String text) {
            return quote(text);
        }
        if (value instanceof Number || value instanceof Boolean) {
            return value.toString();
        }
        if (value instanceof Map<?, ?> map) {
            StringBuilder builder = new StringBuilder();
            builder.append('{');
            Iterator<? extends Map.Entry<?, ?>> iterator = map.entrySet().iterator();
            boolean first = true;
            while (iterator.hasNext()) {
                Map.Entry<?, ?> entry = iterator.next();
                if (!(entry.getKey() instanceof String key)) {
                    continue;
                }
                if (!first) {
                    builder.append(',');
                }
                first = false;
                builder.append(quote(key)).append(':').append(toJson(entry.getValue()));
            }
            builder.append('}');
            return builder.toString();
        }
        if (value instanceof Iterable<?> iterable) {
            StringBuilder builder = new StringBuilder();
            builder.append('[');
            boolean first = true;
            for (Object item : iterable) {
                if (!first) {
                    builder.append(',');
                }
                first = false;
                builder.append(toJson(item));
            }
            builder.append(']');
            return builder.toString();
        }
        throw new AllwrightException("unsupported mobile JSON value: " + value.getClass().getName());
    }

    private static String quote(String value) {
        StringBuilder builder = new StringBuilder(value.length() + 2);
        builder.append('"');
        for (int index = 0; index < value.length(); index += 1) {
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
}

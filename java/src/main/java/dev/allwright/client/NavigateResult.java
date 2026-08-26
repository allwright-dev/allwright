package dev.allwright.client;

public record NavigateResult(
        String url,
        String note,
        String bidiSessionId,
        String mapperTargetId,
        String mapperSessionId,
        String packageVersion
) {}

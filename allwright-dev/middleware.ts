import { NextResponse } from "next/server";
import type { NextRequest } from "next/server";

const GO_IMPORT_PREFIX = "allwright.dev";
const GO_REPO_ROOT = "https://github.com/allwright-dev/allwright";
const GO_SUBDIRECTORY = "go";

function goGetHtml() {
  const goImport = `${GO_IMPORT_PREFIX} git ${GO_REPO_ROOT} ${GO_SUBDIRECTORY}`;
  const goSource = `${GO_IMPORT_PREFIX} ${GO_REPO_ROOT} ${GO_REPO_ROOT}/tree/main/go{/dir} ${GO_REPO_ROOT}/blob/main/go{/dir}/{file}#L{line}`;

  return `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="go-import" content="${goImport}">
    <meta name="go-source" content="${goSource}">
    <meta name="robots" content="noindex">
    <title>${GO_IMPORT_PREFIX}</title>
  </head>
  <body>
    ${GO_IMPORT_PREFIX} Go module metadata
  </body>
</html>`;
}

export function middleware(request: NextRequest) {
  if (request.nextUrl.searchParams.get("go-get") !== "1") {
    return NextResponse.next();
  }

  return new NextResponse(goGetHtml(), {
    headers: {
      "content-type": "text/html; charset=utf-8",
      "cache-control": "public, max-age=300",
    },
  });
}

export const config = {
  matcher: ["/", "/:path*"],
};

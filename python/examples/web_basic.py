from __future__ import annotations

import os

from allwright import launch_firefox, set_server_addr, shutdown

DEFAULT_WEB_URL = "https://themoderninternet.vercel.app"
DEFAULT_WEB_ENTRY_SELECTOR = (
    "xpath=//div[contains(@class,'card')][.//h2[normalize-space()='Form Inputs']]"
    "//button[normalize-space()='Visit page']"
)
DEFAULT_WEB_HEADING_SELECTOR = 'xpath=//h1[text()="Form Inputs"]'
DEFAULT_WEB_HEADING_TEXT = "Form Inputs"


def main() -> None:
    set_server_addr(os.getenv("ALLWRIGHT_SERVER_ADDR", "127.0.0.1:50051"))
    browser = launch_firefox()

    try:
        page = browser.page()
        page.goto(os.getenv("ALLWRIGHT_WEB_URL", DEFAULT_WEB_URL))
        page.click(os.getenv("ALLWRIGHT_WEB_ENTRY_SELECTOR", DEFAULT_WEB_ENTRY_SELECTOR))
        page.wait_for_selector(
            os.getenv("ALLWRIGHT_WEB_HEADING_SELECTOR", DEFAULT_WEB_HEADING_SELECTOR),
        )
        heading = page.text_content(
            os.getenv("ALLWRIGHT_WEB_HEADING_SELECTOR", DEFAULT_WEB_HEADING_SELECTOR)
        )
        expected_heading = os.getenv("ALLWRIGHT_WEB_HEADING_TEXT", DEFAULT_WEB_HEADING_TEXT)
        if expected_heading not in heading.text:
            raise RuntimeError(f"expected heading to contain {expected_heading!r}, got {heading.text!r}")
        print(f"[py-web-basic] heading={heading.text!r}")
    finally:
        browser.close()
        shutdown()


if __name__ == "__main__":
    main()

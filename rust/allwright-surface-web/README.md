# allwright-surface-web

Installable web surface plugin for the allwright engine.

## Protocol Rule

This is a hard rule for all future web-plugin development: Chromium web element operations must execute through WebDriver BiDi. This includes click, hover, focus, fill, key input, selector checks, text reads, highlighting, and screenshots.

CDP is permitted only for Chromium browser and tab lifecycle, bootstrapping the Chromium BiDi mapper, and transporting commands to that mapper. It must not be used to inspect the DOM or dispatch user input in a normal web automation path.

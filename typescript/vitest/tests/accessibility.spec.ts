import { expect, test as base } from "../dist/index.js";
import { shutdown } from "@allwright.dev/core";
import { afterAll } from "vitest";

type SnapshotNode = {
  role: string;
  name: string;
  "aria-ref"?: string;
  children: SnapshotNode[];
};
type Snapshot = { documents: { root: SnapshotNode }[] };
const nodes = (node: SnapshotNode): SnapshotNode[] => [node, ...node.children.flatMap(nodes)];
const test = base.extend({
  allwright: async ({}, use) => {
    await use({ browser: process.env.ALLWRIGHT_TEST_BROWSER === "firefox" ? "firefox" : "chromium" });
  },
});

afterAll(() => shutdown());

// Opt-in: exercises the public fixture, protobuf transport, engine and installed plugin.
// ALLWRIGHT_TEST_BROWSER=chromium|firefox enables a local data-URL fixture.
// ALLWRIGHT_TEST_LIVE_SITE=1 additionally reproduces the reported public-site navigation.
test.skipIf(!process.env.ALLWRIGHT_TEST_BROWSER)("AI snapshot references resolve through the Vitest page fixture", { timeout: 30_000 }, async ({ page }) => {
  if (process.env.ALLWRIGHT_TEST_LIVE_SITE === "1") {
    await page.goto("https://themoderninternet.vercel.app");
    await page.click("xpath=//div[contains(@class,'card')][.//h2[normalize-space()='Form Inputs']]//button[normalize-space()='Visit page']");
  } else {
    await page.goto("data:text/html," + encodeURIComponent("<h1>Form Inputs</h1><label>Name<input></label><button>Save</button>"));
  }
  await expect(page.locator('xpath=//h1[text()="Form Inputs"]')).toHaveText("Form Inputs");
  const snapshot: Snapshot = JSON.parse(await page.accessibilitySnapshot({ format: "json", mode: "ai" }));
  const all = snapshot.documents.flatMap(doc => nodes(doc.root));
  const referenced = all.filter(node => node["aria-ref"]);
  console.log(`AI snapshot: ${all.length} nodes, ${referenced.length} references`);
  expect(referenced.length).toBeGreaterThan(0);
  const heading = all.find(node => node.role === "heading" && node.name === "Form Inputs")!;
  expect(heading["aria-ref"]).toBeTruthy();
  await expect(page.locator(`[aria-ref="${heading["aria-ref"]}"]`)).toHaveText("Form Inputs");
  const repeated: Snapshot = JSON.parse(await page.accessibilitySnapshot({ mode: "ai" }));
  const sameHeading = repeated.documents.flatMap(doc => nodes(doc.root)).find(node => node.role === "heading" && node.name === "Form Inputs")!;
  expect(sameHeading["aria-ref"]).toBe(heading["aria-ref"]);
  const standard: Snapshot = JSON.parse(await page.accessibilitySnapshot());
  expect(standard.documents.flatMap(doc => nodes(doc.root)).some(node => node["aria-ref"])).toBe(false);
});

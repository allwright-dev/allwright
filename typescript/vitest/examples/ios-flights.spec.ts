import { expect, test } from "../dist/index.js";

test("shows a validation alert for an unknown account", { timeout: 60_000 }, async ({ iosApp }) => {
  await iosApp.getByRole("button", { name: "Login", exact: true }).click();
  await expect(iosApp.getByText("Welcome back", { exact: true })).toHaveText("Welcome back");

  await iosApp.getByLabel("Email", { exact: true }).fill("allwright@example.com");
  await iosApp.getByLabel("Password", { exact: true }).fill("not-a-real-password");
  await iosApp.getByRole("button", { name: "Submit", exact: true }).click();

  await expect(iosApp.getByText("No account found. Please sign up first.", { exact: true })).toHaveText(
    "No account found. Please sign up first.",
  );
});

import { test as base, chromium, type BrowserContext, type Worker } from "@playwright/test";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const EXTENSION_PATH = path.resolve(__dirname, "../../extension/dist");

type ExtensionFixtures = {
  context: BrowserContext;
  extensionId: string;
  serviceWorker: Worker;
  setStorage: (items: Record<string, string>) => Promise<void>;
};

export const test = base.extend<ExtensionFixtures>({
  context: [
    async ({}, use) => {
      const context = await chromium.launchPersistentContext("", {
        headless: false,
        args: [
          // Chrome's new headless mode supports extensions; used in CI
          ...(process.env.CI ? ["--headless=new"] : []),
          `--disable-extensions-except=${EXTENSION_PATH}`,
          `--load-extension=${EXTENSION_PATH}`,
          "--no-sandbox",
          "--disable-dev-shm-usage",
        ],
      });
      await use(context);
      await context.close();
    },
    { scope: "test" },
  ],

  serviceWorker: async ({ context }, use) => {
    let [sw] = context.serviceWorkers();
    if (!sw) {
      sw = await context.waitForEvent("serviceworker", { timeout: 10_000 });
    }
    await use(sw);
  },

  extensionId: async ({ serviceWorker }, use) => {
    const extensionId = serviceWorker.url().split("/")[2];
    await use(extensionId);
  },

  // Helper to pre-populate chrome.storage.local via the service worker
  setStorage: async ({ serviceWorker }, use) => {
    await use(async (items: Record<string, string>) => {
      await serviceWorker.evaluate(
        (data) => chrome.storage.local.set(data),
        items
      );
    });
  },
});

export const expect = test.expect;

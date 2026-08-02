const { defineConfig } = require("@playwright/test");

module.exports = defineConfig({
  testDir: "./playwright",
  testMatch: /.*\.spec\.js/,
  fullyParallel: false,
  workers: 1,
  timeout: 120_000,
  expect: {
    timeout: 60_000,
  },
  reporter: [["list"]],
  use: {
    headless: true,
    actionTimeout: 15_000,
    navigationTimeout: 60_000,
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    video: "off",
  },
  projects: [
    {
      name: "chromium",
      use: {
        browserName: "chromium",
      },
    },
    {
      name: "webkit",
      use: {
        browserName: "webkit",
      },
    },
  ],
});

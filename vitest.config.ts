import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "node",
    include: ["tests/frontend/installed.test.tsx", "tests/frontend/linkimport.test.tsx"],
  },
});
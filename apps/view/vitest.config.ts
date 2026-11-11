import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// Frontend-Tests (Vitest + Testing Library) – siehe docs/Testdokumentation.md
export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    setupFiles: ["src/test/setup.ts"],
    include: ["src/**/*.test.{ts,tsx}", "scripts/**/*.test.ts"],
    restoreMocks: true,
  },
});

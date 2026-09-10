import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    server: {
      deps: {
        external: [/actualised_sdk\.win32-x64-msvc\.node/],
      },
    },
  },
});

## Usage

```bash
$ npm install # or pnpm install or yarn install
```

### Learn more on the [Solid Website](https://solidjs.com) and come chat with us on our [Discord](https://discord.com/invite/solidjs)

## Available Scripts

In the project directory, you can run:

### `npm run dev`

Runs the app in the development mode.<br>
Open [http://localhost:5173](http://localhost:5173) to view it in the browser.

### `npm run build`

Builds the app for production to the `dist` folder.<br>
It correctly bundles Solid in production mode and optimizes the build for the best performance.

The build is minified and the filenames include the hashes.<br>
Your app is ready to be deployed!

## Runtime modes

The default build runs the orchestrator locally in browser WASM. This is used for local development and the standalone GitHub Pages demo.

The Cloud Run `build:cloud` script uses the same-origin `/api` endpoints and SSE stream backed by the native SDK, so browser actions and Telegram messages share one persisted company state.

The `build:pages` script emits the `/actualised-ai/` base path for GitHub Pages. Cloud Run and local builds serve from `/`.

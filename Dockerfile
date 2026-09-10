FROM node:20-slim
WORKDIR /app

RUN npm install -g pnpm@9

# Copy workspace files
COPY package.json pnpm-workspace.yaml pnpm-lock.yaml ./
COPY packages/sdk/package.json packages/sdk/
COPY packages/web-ui/package.json packages/web-ui/

# Install dependencies to ensure tsx and hono are available
RUN pnpm install

# Copy the pre-compiled napi binary, typescript configs, and other sdk files
COPY packages/sdk packages/sdk

# Copy the pre-built frontend distribution
COPY packages/web-ui/dist packages/web-ui/dist

ENV PORT=8080
EXPOSE 8080

WORKDIR /app/packages/sdk
CMD ["pnpm", "exec", "tsx", "examples/server.ts"]

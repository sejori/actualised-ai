FROM rust:1.80-slim as builder
WORKDIR /app

# Install Node.js
RUN apt-get update && apt-get install -y curl && \
    curl -fsSL https://deb.nodesource.com/setup_20.x | bash - && \
    apt-get install -y nodejs

# Copy the source code
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY packages/ packages/

# Build napi addon
WORKDIR /app/packages/sdk
RUN npm install
RUN npm run build

# Build Web UI
WORKDIR /app/packages/web-ui
RUN npm install
RUN npm run build

FROM node:20-slim
WORKDIR /app

# Copy SDK and built UI
COPY --from=builder /app/packages/sdk /app/packages/sdk
COPY --from=builder /app/packages/web-ui/dist /app/packages/web-ui/dist

WORKDIR /app/packages/sdk

RUN npm install

ENV PORT=8080
EXPOSE 8080

CMD ["npx", "tsx", "examples/server.ts"]

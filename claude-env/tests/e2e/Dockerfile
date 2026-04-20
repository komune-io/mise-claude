FROM rust:1.79-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM node:20-slim
RUN npm install -g @anthropic-ai/claude-code
COPY --from=builder /app/target/release/claude-env /usr/local/bin/claude-env
WORKDIR /workspace
COPY tests/e2e/scenarios/ /scenarios/
ENTRYPOINT ["/bin/bash"]

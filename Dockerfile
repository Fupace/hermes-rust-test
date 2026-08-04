FROM rust:alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /src
COPY Cargo.toml Cargo.lock* ./
RUN mkdir -p src/api src/db src/models &&     echo 'fn main() {}' > src/main.rs &&     echo '' > src/api/mod.rs && echo '' > src/api/k8s.rs && echo '' > src/api/handlers.rs &&     echo '' > src/db/mod.rs && echo '' > src/db/schema.rs &&     echo '' > src/models/mod.rs && echo '' > src/models/k8s.rs &&     cargo build --release 2>/dev/null || true
COPY . .
RUN touch src/main.rs && cargo build --release

FROM alpine:3.20
RUN apk add --no-cache ca-certificates
WORKDIR /app
COPY --from=builder /src/target/release/hermes-k8s-platform /app/hermes-k8s-platform
COPY static/ /app/static/
EXPOSE 8080
ENTRYPOINT ["/app/hermes-k8s-platform"]

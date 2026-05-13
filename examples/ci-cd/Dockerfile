FROM rust:1.82-alpine AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
RUN cargo build --release

FROM alpine:3.20
RUN adduser -D appuser
COPY --from=builder /app/target/release/my-app /usr/local/bin/my-app
USER appuser
EXPOSE 8080
ENTRYPOINT ["my-app"]

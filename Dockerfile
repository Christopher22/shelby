FROM rust:latest AS builder
WORKDIR /app

COPY Cargo.toml .
COPY Rocket.toml .
COPY src src
COPY assets assets
COPY templates templates
RUN cargo build --release
RUN strip target/release/shelby

FROM gcr.io/distroless/cc-debian12 as release
WORKDIR /app
COPY --from=builder /app/target/release/shelby .
COPY --from=builder /app/assets assets
COPY --from=builder /app/templates templates
COPY --from=builder /app/Rocket.toml .

ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=8000
ENV SHELBY_ASSETS=/app/assets
EXPOSE 8000

CMD ["./shelby", "/data/database.sqlite"]
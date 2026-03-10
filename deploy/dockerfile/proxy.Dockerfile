ARG RUST_VERSION=1.90.0
ARG APP_NAME=proxy

FROM rust:${RUST_VERSION}-alpine AS build
ARG APP_NAME
WORKDIR /app

RUN apk add --no-cache clang lld musl-dev git pkgconfig openssl-dev openssl-libs-static

ENV OPENSSL_STATIC=1

COPY Cargo.toml Cargo.lock ./
COPY src src

RUN cargo build --locked --release && \
    cp ./target/release/$APP_NAME /bin/server

################################################################################
FROM alpine:3.18 AS final

ARG UID=10001
RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    appuser
USER appuser

COPY --from=build /bin/server /bin/

EXPOSE 8080

CMD ["/bin/server"]

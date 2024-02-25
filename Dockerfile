FROM rust:1.72.0-alpine3.18 as app-build

WORKDIR /build

RUN mkdir -p /build/static && \
    apk add nodejs npm musl-dev elfutils xz wget pkgconfig libressl-dev perl make && \
    wget https://github.com/upx/upx/releases/download/v4.0.2/upx-4.0.2-amd64_linux.tar.xz && \
    unxz upx-4.0.2-amd64_linux.tar.xz && tar xvf upx-4.0.2-amd64_linux.tar && \
    cp upx-4.0.2-amd64_linux/upx /usr/bin/upx && chmod +x /usr/bin/upx

COPY . /build

RUN cargo test && \
    cargo build --release && \
    eu-elfcompress ./target/release/qb && \
    strip ./target/release/qb && \
    upx -9 --lzma ./target/release/qb && \
    chmod +x ./target/release/qb

FROM alpine:3.18.3 AS runtime
WORKDIR /app
RUN apk add libressl-dev && \
    adduser -h /app -D quarterback && \
    chmod 700 /app && \
    chown -R quarterback: /app

COPY --from=app-build /build/target/release/qb /app/qb
COPY --from=app-build /build/.env /app/.env
RUN chown -R quarterback: /app && chmod +x /app/qb 

USER quarterback

CMD ["/app/qb"]

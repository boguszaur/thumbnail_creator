FROM ekidd/rust-musl-builder:stable as build

ADD . /home/rust/src/
RUN sudo chown -R rust:rust . && cargo build --release

FROM alpine:3.10

ENV    APP_LISTEN_IP    0.0.0.0
ENV    APP_LISTEN_PORT  8080
ENV    APP_SHUTDOWN_TIMEOUT 30
ENV    APP_MAX_CONTENT_LENGTH   3000000
ENV    APP_CHECK_MIME_TYPE  true
ENV    APP_MAX_URLS_IN_SINGLE_REQ   70
ENV    APP_HTTP_CLIENT_TIMEOUT  5
ENV    APP_THUMBNAIL_WIDTH  100
ENV    APP_THUMBNAIL_HEIGHT 100
ENV    APP_THUMBNAIL_EXACT_SIZE true
ENV    APP_STORAGE_BASE_DIR /images/out
ENV    APP_THUMBNAIL_EXTENSION  JPG
ENV    APP_LOG_LEVEL info,actix_web=debug

RUN apk update && \
    apk add ca-certificates && \
    update-ca-certificates && \
    rm -rf /var/cache/apk/* && \
    adduser -D -u 1000 user && \
    mkdir /service
WORKDIR /service
COPY --from=build /home/rust/src/target/x86_64-unknown-linux-musl/release/thumbnail_creator .
RUN chown -R user:user /service
USER user

EXPOSE $APP_LISTEN_PORT
VOLUME $APP_STORAGE_BASE_DIR
CMD ["./thumbnail_creator"]
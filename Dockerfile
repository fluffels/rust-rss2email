FROM rust:1-alpine3.13 AS builder

RUN apk add libc-dev openssl-dev

RUN adduser -D builder
WORKDIR /home/builder

COPY Cargo.toml Cargo.lock ./
RUN cargo update

COPY ./src ./src
RUN cargo build --release


FROM alpine:3.13

RUN apk add openssl

RUN adduser -D app
WORKDIR /home/app

COPY --from=builder /home/builder/target/release .

FROM rust:1.91.0-alpine AS builder
RUN apk add --no-cache pkgconfig openssl musl-dev libressl-dev
COPY . /app
WORKDIR /app
RUN cargo b -r

FROM alpine:3.20.3
RUN apk add --no-cache tzdata
ENV TZ=Europe/Rome
RUN mkdir /app
COPY --from=builder /app/target/release/psg-calendar-to-ballbreaker /app
WORKDIR /app
ENTRYPOINT ["./psg-calendar-to-ballbreaker"]

FROM rust:1.86.0-slim AS base

FROM base AS builder

RUN apt update ;\
  apt upgrade -y ;\
  apt install openssl pkg-config libssl-dev file make git -y

RUN mkdir /build

COPY ./ /build/

RUN cd /build ;\
  cargo build --release


FROM base AS ds_proxy

COPY --from=builder /build/target/release/ds_proxy /dsproxy/ds_proxy

RUN groupadd -r dsproxy -g 1000 && useradd -r -g dsproxy -u 1000 dsproxy
RUN chown -R dsproxy:dsproxy /dsproxy
USER dsproxy

ENTRYPOINT ["/dsproxy/ds_proxy"]


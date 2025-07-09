
# syntax=docker/dockerfile:1

FROM rust:slim AS builder


RUN apt-get update && \
    apt-get install -y build-essential pkg-config  bzip2 libudev-dev llvm libclang-dev protobuf-compiler libssl-dev ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

COPY ./agave ./agave

WORKDIR /agave

RUN ./scripts/cargo-install-all.sh .

FROM rust:slim

EXPOSE 8899/tcp
# RPC pubsub
EXPOSE 8900/tcp
# entrypoint
EXPOSE 8001/tcp
# (future) bank service
EXPOSE 8901/tcp
# bank service
EXPOSE 8902/tcp
# faucet
EXPOSE 9900/tcp
# tvu
EXPOSE 8000/udp
# gossip
EXPOSE 8001/udp
# tvu_quic
EXPOSE 8002/udp
# tpu
EXPOSE 8003/udp
# tpu_forwards
EXPOSE 8004/udp
# retransmit
EXPOSE 8005/udp
# repair
EXPOSE 8006/udp
# serve_repair
EXPOSE 8007/udp
# broadcast
EXPOSE 8008/udp
# tpu_vote
EXPOSE 8009/udp

COPY --from=builder /agave/bin/* /usr/local/bin/
RUN apt-get update && apt-get install ca-certificates -y

CMD ["solana-test-validator"]

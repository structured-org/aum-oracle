FROM golang:1.23.6-alpine AS builder
WORKDIR /app
COPY . .

WORKDIR /app/aum_messenger
RUN go mod download

RUN go build -a -o aum-messenger .

FROM alpine:latest
WORKDIR /app
COPY --from=builder /app/aum_messenger/aum-messenger .

ENTRYPOINT ["./aum-messenger"]

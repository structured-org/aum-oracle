FROM golang:1.23.6-alpine AS builder
WORKDIR /aum_messenger
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN go build -a -o aum-messenger .

FROM alpine:latest
WORKDIR /aum_messenger
COPY --from=builder /aum_messenger/aum-messenger .

ENTRYPOINT ["./aum-messenger"]

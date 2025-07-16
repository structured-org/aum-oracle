FROM golang:1.23.6-alpine AS builder
WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN go build -a -o aum-oracle .

FROM alpine:latest
WORKDIR /app
COPY --from=builder /app/aum-oracle .

ENTRYPOINT ["./aum-oracle"]
CMD ["--config", "/app/config.yaml"]

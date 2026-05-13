FROM golang:1.22-alpine AS builder
WORKDIR /app
COPY . .
RUN go build -o /api ./main.go

FROM alpine:3.19
COPY --from=builder /api /usr/local/bin/api
EXPOSE 8080
ENTRYPOINT ["api"]

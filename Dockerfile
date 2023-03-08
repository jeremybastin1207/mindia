FROM golang:latest as builder
WORKDIR /app

ENV GO111MODULE=on

COPY ./go.mod .
COPY ./go.sum .

RUN go mod download

COPY . .

RUN CGO_ENABLED=0 go build -o . ./...

FROM alpine:latest as runner
WORKDIR /app

COPY --from=builder /app/mindia .
COPY --from=builder /app/.env .

RUN chmod +x ./mindia

CMD ["./mindia"]
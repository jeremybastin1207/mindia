FROM golang:latest as builder
WORKDIR /app

ENV CGO_ENABLED=0

COPY ./go.mod .
COPY ./go.sum .
RUN go mod download

COPY . .

RUN go build -o ./mindia

FROM alpine:latest as runner
WORKDIR /app

RUN addgroup --system --gid 1001 runner
RUN adduser --system --uid 1001 runner

RUN mkdir ./data
RUN chown -R runner:runner ./data

COPY --from=builder --chown=runner:runner /app/config.yaml .
COPY --from=builder --chown=runner:runner /app/mindia .

RUN chmod +x ./mindia

USER runner

CMD ["./mindia"]
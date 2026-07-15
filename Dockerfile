FROM golang:1.26.5-trixie AS builder
WORKDIR /src

COPY src/go.mod src/go.sum ./
RUN go mod download

COPY src/*.go ./
RUN CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -trimpath -ldflags='-s -w' -o /out/eks-version-exporter .
FROM debian:trixie-slim AS kubectl
ARG KUBECTL_VERSION=stable

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

RUN set -eux; \
    if [ "$KUBECTL_VERSION" = "stable" ]; then \
      KUBECTL_VERSION="$(curl -fsSL https://dl.k8s.io/release/stable.txt)"; \
    fi; \
    curl -fsSLo /kubectl "https://dl.k8s.io/release/${KUBECTL_VERSION}/bin/linux/amd64/kubectl"; \
    chmod +x /kubectl

FROM debian:trixie-slim
ARG APP=/usr/src/app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates tzdata \
    && rm -rf /var/lib/apt/lists/*

ENV TZ=Etc/UTC
ENV APP_USER=dfds

RUN groupadd "$APP_USER" \
    && useradd -g "$APP_USER" "$APP_USER" \
    && mkdir -p "$APP"

WORKDIR $APP
COPY --from=kubectl /kubectl /usr/local/bin/kubectl
COPY --chown=$APP_USER:$APP_USER --from=builder /out/eks-version-exporter ./eks-version-exporter

USER $APP_USER

EXPOSE 8080
CMD ["./eks-version-exporter"]

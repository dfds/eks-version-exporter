FROM rust:1.67 as builder

RUN USER=root cargo new --bin deps-caching
WORKDIR ./deps-caching
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
RUN cargo build --release
RUN rm src/*.rs

COPY . .

RUN rm ./target/release/deps/eks_version_exporter*
RUN cargo build --release --offline

# App assembling
FROM debian:bullseye-slim
ARG APP=/usr/src/app

RUN apt-get update \
    && apt-get install -y ca-certificates tzdata curl \
    && rm -rf /var/lib/apt/lists/*

RUN curl -LO "https://storage.googleapis.com/kubernetes-release/release/$(curl -s https://storage.googleapis.com/kubernetes-release/release/stable.txt)/bin/linux/amd64/kubectl" \
    && chmod +x ./kubectl \
    && mv ./kubectl /usr/local/bin/kubectl

EXPOSE 8080

ENV TZ=Etc/UTC
ENV APP_USER=dfds

RUN groupadd $APP_USER \
    && useradd -g $APP_USER $APP_USER \
    && mkdir -p ${APP}

COPY --from=builder /deps-caching/target/release/eks-version-exporter ${APP}/eks-version-exporter

RUN chown -R $APP_USER:$APP_USER ${APP}

USER $APP_USER
WORKDIR ${APP}

CMD ["./eks-version-exporter"]
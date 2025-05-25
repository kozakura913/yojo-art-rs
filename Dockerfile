from public.ecr.aws/docker/library/rust:latest as build_app
RUN apt-get update && apt-get install -y clang musl-dev pkg-config nasm mold git
ENV CARGO_HOME=/var/cache/cargo
WORKDIR /app
COPY avif-decoder_dep ./avif-decoder_dep
COPY assets ./assets
COPY utils/src ./utils/src
COPY utils/Cargo.toml ./utils/Cargo.toml
COPY src ./src
COPY Cargo.toml ./Cargo.toml
RUN --mount=type=cache,target=/var/cache/cargo --mount=type=cache,target=/app/target cargo build --release
RUN --mount=type=cache,target=/app/target mv /app/target/release/yojo-art-rs /app/yojo-art-rs

from public.ecr.aws/docker/library/debian:sid-slim
ARG UID="991"
ARG GID="991"
RUN groupadd -g "${GID}" cherrypick && useradd -l -u "${UID}" -g "${GID}" -m -d /cherrypick cherrypick
WORKDIR /cherrypick/
USER cherrypick
COPY --chown=cherrypick:cherrypick --from=build_app /app/yojo-art-rs /cherrypick/yojo-art-rs
CMD ["/cherrypick/yojo-art-rs"]

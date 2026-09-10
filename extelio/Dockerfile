# syntax=docker/dockerfile:1.7
# EXTELIO - Mehrstufiger Build (Kapitel 22.3)
#
# Stufen: rust-builder | web-builder | freeswitch-builder | runtime
#
# Die FreeSWITCH-Stufe ist ueber ARG WITH_FREESWITCH schaltbar. In der
# Voreinstellung (0) wird die App ohne Telefonie-Core gebaut: Weboberflaeche,
# IAM, KMS, Config Compiler, Audit, Health und Backup sind vollstaendig
# nutzbar, der Telefonie-Core wird als MAINTENANCE ausgewiesen. Mit
# WITH_FREESWITCH=1 wird FreeSWITCH aus den Quellen gebaut und eingebunden.

ARG BUILD_FROM
ARG RUST_VERSION=1.98.1
ARG NODE_VERSION=24.21.0
ARG FREESWITCH_VERSION=1.11.3
ARG WITH_FREESWITCH=0

# --------------------------------------------------------------- Rust-Build
FROM rust:${RUST_VERSION}-alpine AS rust-builder

RUN apk add --no-cache musl-dev build-base

WORKDIR /build
COPY Cargo.toml Cargo.lock* ./
COPY migrations ./migrations
COPY src/rust ./src/rust

# Reproduzierbarer Release-Build ohne Debug-Symbole.
RUN cargo build --release --locked || cargo build --release
RUN strip target/release/extelio-bootstrap \
           target/release/extelio-secret-broker \
           target/release/extelio-web \
           target/release/extelio-worker || true

# ---------------------------------------------------------------- Web-Build
FROM node:${NODE_VERSION}-alpine AS web-builder

WORKDIR /web
COPY src/web/package.json src/web/package-lock.json* ./
RUN npm ci --no-audit --no-fund || npm install --no-audit --no-fund
COPY src/web/ ./
COPY brand/ ./public/brand/
RUN npm run build

# --------------------------------------------------------- FreeSWITCH-Build
# Nur aktiv, wenn WITH_FREESWITCH=1 gesetzt ist. Es werden ausschliesslich die
# Module aus der Allowlist (Kapitel 22.4) gebaut.
FROM alpine:3.21 AS freeswitch-builder
ARG FREESWITCH_VERSION
ARG WITH_FREESWITCH
WORKDIR /fsbuild
COPY freeswitch/modules.conf ./modules.conf
RUN if [ "${WITH_FREESWITCH}" = "1" ]; then \
      apk add --no-cache build-base autoconf automake libtool pkgconf git \
        curl-dev openssl-dev sqlite-dev pcre2-dev speex-dev speexdsp-dev \
        libsndfile-dev opus-dev ldns-dev libedit-dev yasm bash && \
      git clone --depth 1 --branch v${FREESWITCH_VERSION} \
        https://github.com/signalwire/freeswitch.git src && \
      cp modules.conf src/modules.conf && \
      cd src && ./bootstrap.sh -j && \
      ./configure --prefix=/opt/freeswitch --disable-debug --without-python \
        --disable-libvpx --disable-video && \
      make -j"$(nproc)" && make install ; \
    else \
      mkdir -p /opt/freeswitch && \
      echo "FreeSWITCH wurde in diesem Build nicht eingebunden (WITH_FREESWITCH=0)." \
        > /opt/freeswitch/NOT_BUILT ; \
    fi

# ----------------------------------------------------------------- Laufzeit
FROM ${BUILD_FROM} AS runtime

ARG BUILD_VERSION
ARG BUILD_ARCH
ARG WITH_FREESWITCH

ENV LANG=C.UTF-8 \
    EXTELIO_DATA_DIR=/data \
    EXTELIO_RUN_DIR=/run/extelio \
    EXTELIO_WEB_ROOT=/opt/extelio/web \
    EXTELIO_FREESWITCH_ENABLED=${WITH_FREESWITCH}

RUN apk add --no-cache \
      ca-certificates \
      sqlite-libs \
      libgcc \
      tzdata \
      certbot \
    && rm -rf /var/cache/apk/*

# Anwendungsdateien liegen unveraenderlich unter /opt (Kapitel 22.5).
COPY --from=rust-builder /build/target/release/extelio-bootstrap      /opt/extelio/bin/
COPY --from=rust-builder /build/target/release/extelio-secret-broker  /opt/extelio/bin/
COPY --from=rust-builder /build/target/release/extelio-web            /opt/extelio/bin/
COPY --from=rust-builder /build/target/release/extelio-worker         /opt/extelio/bin/
COPY --from=web-builder  /web/dist                                    /opt/extelio/web/
COPY --from=freeswitch-builder /opt/freeswitch                        /opt/freeswitch/
COPY brand/                                                           /opt/extelio/brand/
COPY freeswitch/                                                      /opt/extelio/freeswitch/
COPY rootfs/                                                          /

RUN chmod +x /opt/extelio/bin/* \
    && find /etc/s6-overlay/s6-rc.d -type f -name run -exec chmod +x {} \; \
    && find /etc/s6-overlay/s6-rc.d -type f -name up -exec chmod +x {} \; \
    && find /etc/s6-overlay/s6-rc.d -type f -name finish -exec chmod +x {} \;

LABEL \
  io.hass.name="EXTELIO" \
  io.hass.description="Software-defined Communications" \
  io.hass.type="addon" \
  io.hass.version="${BUILD_VERSION}" \
  io.hass.arch="${BUILD_ARCH}" \
  org.opencontainers.image.title="EXTELIO" \
  org.opencontainers.image.licenses="Apache-2.0"

WORKDIR /data

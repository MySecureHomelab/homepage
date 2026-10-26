FROM rust:1-slim-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --release --locked

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /src/target/release/homepage /homepage
ENV HOMEPAGE_CONFIG=/config/config.yaml
EXPOSE 3000
ENTRYPOINT ["/homepage"]

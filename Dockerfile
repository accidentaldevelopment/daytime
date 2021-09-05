FROM rust:1.54.0 as build
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc:nonroot
EXPOSE 13
COPY --from=build /app/target/release/daytime /
CMD ["/daytime"]

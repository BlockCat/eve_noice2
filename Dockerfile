# FROM rust as build
# ENV PKG_CONFIG_ALLOW_CROSS=1

# RUN cargo install sqlx-cli --no-default-features --features sqlite

# WORKDIR /usr/src/noice2

# COPY . .

# RUN cargo install --path .
# RUN sqlx database setup

# RUN cp /usr/local/cargo/bin/noice2 /usr/local/bin/noice2
# RUN cp /usr/src/noice2/database.db /usr/local/bin/database.db

# ENV DATABASE_URL="sqlite:/usr/local/bin/database.db"

# CMD ["noice2"]



FROM rust as build
ENV PKG_CONFIG_ALLOW_CROSS=1

RUN cargo install sqlx-cli --no-default-features --features sqlite

WORKDIR /usr/src/noice2

COPY . .

RUN sqlx database setup
RUN cargo install --path .

FROM debian:12

COPY --from=build /usr/local/cargo/bin/noice2 /app/noice2
COPY --from=build /usr/src/noice2/database.db /app/data/database.db

ENV DATABASE_URL="sqlite:/app/data/database.db"

VOLUME /app/data

CMD ["/app/noice2"]

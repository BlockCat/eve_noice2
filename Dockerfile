FROM rust as build
ENV PKG_CONFIG_ALLOW_CROSS=1

RUN cargo install sqlx-cli --no-default-features --features sqlite

WORKDIR /usr/src/noice2

COPY . .

RUN cargo install --path .
RUN sqlx database setup

RUN cp /usr/local/cargo/bin/noice2 /usr/local/bin/noice2
RUN cp /usr/src/noice2/database.db /usr/local/bin/database.db

ENV DATABASE_URL="sqlite:/usr/local/bin/database.db"

CMD ["noice2"]
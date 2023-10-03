# Noice2


## Install

1. Download tranquuility SDE: https://developers.eveonline.com/resource
2. Extract the `sde` folder to the `data` folder (you'll see `/data/sde`)
3. Create a database `cargo sqlx database create`
4. Update SDE migration `cargo run --package noice2 --example write_migration  --release`
5. Run migrations `cargo sqlx migrate run`

## Update SDE


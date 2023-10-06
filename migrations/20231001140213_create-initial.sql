
CREATE TABLE IF NOT EXISTS eve_region (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS eve_system (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    region_id INTEGER NOT NULL REFERENCES eve_region(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS eve_groups (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS eve_market_groups (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS eve_items (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    published BOOLEAN NOT NULL DEFAULT FALSE,
    group_id INTEGER NOT NULL REFERENCES eve_groups(id) ON DELETE CASCADE,
    market_group_id INTEGER REFERENCES eve_market_groups(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS update_log (
    id INTEGER PRIMARY KEY,
    date DATE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    region_id INTEGER NOT NULL REFERENCES eve_region(id) ON DELETE CASCADE,
    finished BOOLEAN NOT NULL DEFAULT FALSE,
    UNIQUE(date, region_id)
);

CREATE TABLE IF NOT EXISTS market_history (
    id INTEGER PRIMARY KEY,
    date DATE NOT NULL,
    item_id INTEGER NOT NULL REFERENCES eve_items(id) ON DELETE CASCADE,
    region_id INTEGER NOT NULL REFERENCES eve_region(id) ON DELETE CASCADE,
    low_price INTEGER NOT NULL,
    high_price INTEGER NOT NULL,
    average_price INTEGER NOT NULL,
    order_count INTEGER NOT NULL,
    volume INTEGER NOT NULL,
    created DATE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(date, item_id, region_id)
);

CREATE TABLE IF NOT EXISTS market_orders (
    id INTEGER PRIMARY KEY,
    buy_order BOOLEAN NOT NULL,
    issued DATE NOT NULL,
    expiry DATE NOT NULL,
    order_id INTEGER NOT NULL,
    price REAL NOT NULL,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    item_id INTEGER NOT NULL REFERENCES eve_items(id) ON DELETE CASCADE,
    system_id INTEGER NOT NULL REFERENCES eve_system(id) ON DELETE CASCADE,
    created DATE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    volume_remain INTEGER NOT NULL,
    volume_total INTEGER NOT NULL,
    UNIQUE(order_id, issued, volume_remain)
);

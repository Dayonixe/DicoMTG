CREATE TABLE reference_prices (
    uuid TEXT PRIMARY KEY NOT NULL CHECK (length(uuid) > 0),
    amount_cents INTEGER NOT NULL CHECK (amount_cents >= 0),
    currency TEXT NOT NULL CHECK (currency = 'EUR'),
    as_of_date TEXT NOT NULL CHECK (length(as_of_date) = 10)
);

use rusqlite::{Connection, Result};

pub fn migrate_to_string_storage(conn: &Connection) -> Result<()> {
    // Begin transaction
    conn.execute("BEGIN TRANSACTION", [])?;
    
    // Create new tables with string storage for floats
    conn.execute(
        "CREATE TABLE IF NOT EXISTS contracts_new (
            id INTEGER PRIMARY KEY,
            side TEXT NOT NULL,
            strike_price_cents INTEGER NOT NULL,
            quantity_str TEXT NOT NULL,
            expires INTEGER NOT NULL,
            premium_str TEXT NOT NULL,
            created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
        )",
        [],
    )?;
    
    conn.execute(
        "CREATE TABLE IF NOT EXISTS premium_history_new (
            id INTEGER PRIMARY KEY,
            product_key TEXT NOT NULL,
            side TEXT NOT NULL,
            strike_price_cents INTEGER NOT NULL,
            expires INTEGER NOT NULL,
            premium_str TEXT NOT NULL,
            timestamp INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
            UNIQUE(product_key, timestamp)
        )",
        [],
    )?;
    
    // Migrate existing data if tables exist
    let tables_exist: bool = conn
        .prepare("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='contracts'")?
        .query_row([], |row| row.get::<_, i64>(0))
        .map(|count| count > 0)
        .unwrap_or(false);
    
    if tables_exist {
        // Migrate contracts table
        conn.execute(
            "INSERT INTO contracts_new (id, side, strike_price_cents, quantity_str, expires, premium_str, created_at)
             SELECT id, side, CAST(strike_price * 100 AS INTEGER), 
                    printf('%.8f', quantity), expires, printf('%.8f', premium), created_at
             FROM contracts",
            [],
        )?;
        
        // Migrate premium_history table
        conn.execute(
            "INSERT INTO premium_history_new (id, product_key, side, strike_price_cents, expires, premium_str, timestamp)
             SELECT id, product_key, side, CAST(strike_price * 100 AS INTEGER), 
                    expires, printf('%.8f', premium), timestamp
             FROM premium_history",
            [],
        )?;
        
        // Drop old tables
        conn.execute("DROP TABLE contracts", [])?;
        conn.execute("DROP TABLE premium_history", [])?;
    }
    
    // Rename new tables
    conn.execute("ALTER TABLE contracts_new RENAME TO contracts", [])?;
    conn.execute("ALTER TABLE premium_history_new RENAME TO premium_history", [])?;
    
    // Create indexes
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_contracts_created_at ON contracts(created_at)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_premium_history_product ON premium_history(product_key, timestamp)",
        [],
    )?;
    
    // Commit transaction
    conn.execute("COMMIT", [])?;
    
    Ok(())
}
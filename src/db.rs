use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{Connection, Result};
use std::sync::Arc;

pub type DbPool = Arc<Pool<SqliteConnectionManager>>;

pub fn create_pool() -> Result<DbPool, Box<dyn std::error::Error>> {
    let manager = SqliteConnectionManager::file("contracts.db");
    let pool = Pool::new(manager)?;
    
    // Initialize database schema using a connection from the pool
    let conn = pool.get()?;
    init_db(&conn)?;
    
    Ok(Arc::new(pool))
}

// Initialize the SQLite database and creates the tables if they don't exist.
pub fn init_db(conn: &Connection) -> Result<()> {
    // Create contracts table with string storage for floats
    conn.execute(
        "CREATE TABLE IF NOT EXISTS contracts (
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
    
    // Create premium history table for tracking price movements
    conn.execute(
        "CREATE TABLE IF NOT EXISTS premium_history (
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
    
    // Create index for efficient queries
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_contracts_created_at ON contracts(created_at)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_premium_history_product ON premium_history(product_key, timestamp)",
        [],
    )?;
    
    Ok(())
}
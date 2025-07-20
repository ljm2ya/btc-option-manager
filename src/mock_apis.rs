// Actix-web for web server functionality.
use actix_web::{web, App, HttpResponse, HttpServer, Responder};

// Defines the request structure for the '/iv' endpoint.
#[derive(serde::Deserialize)]
struct IvRequest {
    side: String,
    strike_price: f64,
    expire: String,
}

// Mock endpoint for the liquidity pool quantity.
async fn get_pool_qty() -> impl Responder {
    HttpResponse::Ok().json(5.0)
}

// Mock endpoint for the current asset price.
async fn get_price() -> impl Responder {
    HttpResponse::Ok().json(100500.0)
}

// Mock endpoint for calculating Implied Volatility (IV).
async fn get_iv(req: web::Query<IvRequest>) -> impl Responder {
    // Simulate a "volatility smile" where IV increases based on distance from the current price.
    let iv = 0.5 + (req.strike_price - 100500.0).abs() / 100500.0 * 0.1;
    HttpResponse::Ok().json(iv)
}

// Main function for the mock server.
// Runs a separate server on port 8081 to provide simulated financial data.
pub async fn mock_server() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(web::resource("/pool").route(web::get().to(get_pool_qty)))
            .service(web::resource("/price").route(web::get().to(get_price)))
            .service(web::resource("/iv").route(web::get().to(get_iv)))
    })
    .bind("127.0.0.1:8081")?
    .run()
    .await
} 
// Actix-web for web server functionality.
use actix_web::{web, App, HttpResponse, HttpServer, Responder};

// Defines the request structure for the '/iv' endpoint.
#[derive(serde::Deserialize)]
struct IvRequest {
    #[allow(dead_code)]
    side: String,
    strike_price: f64,
    #[allow(dead_code)]
    expire: String,
}

// Mock endpoint for calculating Implied Volatility (IV).
// This is the only remaining mock endpoint as IV data comes from Deribit,
// but we keep this as a fallback when Deribit data is unavailable.
async fn get_iv(req: web::Query<IvRequest>) -> impl Responder {
    // Simulate a "volatility smile" where IV increases based on distance from the current price.
    // Using a base price of 50000 as a reasonable BTC price estimate
    let base_price = 50000.0;
    let iv = 0.5 + (req.strike_price - base_price).abs() / base_price * 0.1;
    HttpResponse::Ok().json(iv)
}

// Main function for the mock server.
// Runs a separate server on port 8081 to provide fallback IV data.
// Pool data now comes from Mutiny wallet and price data from gRPC oracle.
pub async fn mock_server() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(web::resource("/iv").route(web::get().to(get_iv)))
    })
    .bind("0.0.0.0:8081")?
    .run()
    .await
} 
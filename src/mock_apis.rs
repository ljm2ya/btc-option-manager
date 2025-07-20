use actix_web::{web, App, HttpResponse, HttpServer, Responder};

#[derive(serde::Deserialize)]
struct IvRequest {
    side: String,
    strike_price: f64,
    expire: String,
}

async fn get_pool_qty() -> impl Responder {
    HttpResponse::Ok().json(5.0)
}

async fn get_price() -> impl Responder {
    HttpResponse::Ok().json(100500.0)
}

async fn get_iv(req: web::Query<IvRequest>) -> impl Responder {
    let iv = 0.5 + (req.strike_price - 100500.0).abs() / 100500.0 * 0.1;
    HttpResponse::Ok().json(iv)
}

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
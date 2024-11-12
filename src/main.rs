use actix_web::{get, post, web, HttpResponse, Responder};
use nanoid::nanoid;
use sqlx::{Error, PgPool};
use url::Url;

async fn initialize_db(pool: &PgPool) {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS urls (
            id VARCHAR(6) PRIMARY KEY,
            url VARCHAR NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await
    .expect("Failed to create table");
}

#[get("/")]
async fn hello_world() -> impl Responder {
    HttpResponse::Ok().body("Hello World!")
}

#[get("/{id}")]
async fn redirect(
    id: web::Path<String>,
    pool: web::Data<PgPool>,
) -> Result<impl Responder, actix_web::Error> {
    let id = id.into_inner();
    let url: (String,) = sqlx::query_as("SELECT url FROM urls WHERE id = $1")
        .bind(id)
        .fetch_one(pool.get_ref())
        .await
        .map_err(|e| match e {
            Error::RowNotFound => actix_web::error::ErrorNotFound("URL not found"),
            _ => actix_web::error::ErrorInternalServerError("Database error"),
        })?;

    Ok(HttpResponse::Found()
        .append_header(("Location", url.0))
        .finish())
}

#[post("/")]
async fn shorten(url: String, pool: web::Data<PgPool>) -> Result<impl Responder, actix_web::Error> {
    let id = nanoid!(6);
    let parsed_url =
        Url::parse(&url).map_err(|_| actix_web::error::ErrorUnprocessableEntity("Invalid URL"))?;

    sqlx::query("INSERT INTO urls (id, url) VALUES ($1, $2)")
        .bind(&id)
        .bind(parsed_url.as_str())
        .execute(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    Ok(HttpResponse::Ok().body(format!("https://compresseverything.shuttleapp.rs/{}", id)))
}

#[shuttle_runtime::main]
async fn main(
    #[shuttle_shared_db::Postgres] pool: PgPool,
) -> shuttle_actix_web::ShuttleActixWeb<impl FnOnce(&mut web::ServiceConfig) + Send + Clone + 'static>
{
    initialize_db(&pool).await;

    let config = move |cfg: &mut web::ServiceConfig| {
        cfg.app_data(web::Data::new(pool.clone()))
            .service(hello_world)
            .service(redirect)
            .service(shorten);
    };

    Ok(config.into())
}

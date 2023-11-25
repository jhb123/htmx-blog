use rocket::{Rocket, fairing::{self, AdHoc}, Build, error, get, serde::json::Json, routes};
use rocket_db_pools::{sqlx, Database, Connection};
use serde::{Deserialize, Serialize};
use sqlx::migrate;

type Result<T, E = rocket::response::Debug<sqlx::Error>> = std::result::Result<T, E>;

#[derive(Database)]
#[database("sqlx")]
pub struct SiteDatabase(sqlx::mysql::MySqlPool);

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("SQLx Stage", |rocket| async {
        rocket.attach(SiteDatabase::init())
            .attach(AdHoc::try_on_ignite("SQLx Migrations", run_migrations))
            .mount("/db", routes![users])
    })
}


#[derive(Debug, Clone, Deserialize, Serialize, sqlx::FromRow)]
#[serde(crate = "rocket::serde")]
pub struct UserData {
    id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    session: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    privilege: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[get("/users")]
async fn users(mut db: Connection<SiteDatabase>) -> Result<Json<Vec<UserData>>>{ 
    let res: Vec<UserData>  = sqlx::query_as::<_, UserData>("SELECT * FROM users").fetch_all(&mut *db).await?;
    Ok(Json(res))
}

async fn run_migrations(rocket: Rocket<Build>) -> fairing::Result {
    match SiteDatabase::fetch(&rocket) {
        Some(db) => match migrate!("db/migrations").run(&**db).await {
            Ok(_) => Ok(rocket),
            Err(e) => {
                error!("Failed to initialize SQLx database: {}", e);
                Err(rocket)
            }
        }
        None => Err(rocket),
    }
}
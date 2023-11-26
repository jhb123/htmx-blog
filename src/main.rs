use rocket::fairing::AdHoc;
use rocket::{routes, launch, get};
use htmx_blog::auth::api;
use htmx_blog::db;
use htmx_blog::config::AppConfig;

#[launch]
async fn rocket() ->  _ {

    rocket::build()
        .mount("/", routes![index])
        .attach(db::stage())
        .attach(api::stage())
        .attach(AdHoc::config::<AppConfig>())

}

#[get("/")]
fn index() -> &'static str {
    "Server Alive"
}


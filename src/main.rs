use rocket::{routes, launch, get};
use htmx_blog::auth::api;
use htmx_blog::db;

#[launch]
async fn rocket() ->  _ {

    rocket::build()
        .mount("/", routes![index])
        .attach(db::stage())
        .attach(api::stage())

}

#[get("/")]
fn index() -> &'static str {
    "Hello, World"
}


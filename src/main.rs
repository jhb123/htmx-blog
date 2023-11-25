use rocket::{routes, launch, get};
use htmx_blog::auth::api::stage;


#[launch]
async fn rocket() ->  _ {

    rocket::build()
        .mount("/", routes![index])
        .attach(stage())

}

#[get("/")]
fn index() -> &'static str {
    "Hello, World"
}


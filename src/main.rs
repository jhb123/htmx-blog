use std::path::{PathBuf, Path};

use rocket::fairing::AdHoc;
use rocket::{routes, launch, get, catch, catchers};
use rocket::fs::NamedFile;
use rocket_dyn_templates::{Template, context};

use htmx_blog::auth::api;
use htmx_blog::auth::api::User;
use htmx_blog::{db, writing};
use htmx_blog::config::AppConfig;

#[launch]
async fn rocket() ->  _ {

    rocket::build()
        .mount("/", routes![index_admin, static_resources, js_resources])
        .attach(db::stage())
        .attach(api::stage())
        .attach(writing::stage())
        .attach(AdHoc::config::<AppConfig>())
        .attach(Template::fairing())
        .register("/", catchers![index])


}

// #[get("/", rank = 1)]
// fn index() -> Template {
//     Template::render("index", context! { title: "Hello, World", admin: false })
// }

#[catch(401)]
fn index() -> Template { 
        Template::render("index", context! { admin: false })
}

#[get("/", rank=1)]
fn index_admin(_user: User) -> Template { 
        Template::render("index", context! { admin: true })
}


#[get("/js/<path..>", rank = 2)]
async fn js_resources(path: PathBuf) -> Option<NamedFile> {
    let base_path = Path::new("./js/");
    let full_path = base_path.join(path);
    NamedFile::open(full_path).await.ok()
}

#[get("/<path..>", rank = 3)]
async fn static_resources(path: PathBuf) -> Option<NamedFile> {
    let base_path = Path::new("./static/");
    let full_path = base_path.join(path);
    NamedFile::open(full_path).await.ok()
}

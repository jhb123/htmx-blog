use std::path::{PathBuf, Path};

use rocket::fairing::AdHoc;
use rocket::{routes, launch, get};
use rocket::fs::NamedFile;
use rocket_dyn_templates::{Template, context};

use htmx_blog::auth::api;
use htmx_blog::auth::api::User;
use htmx_blog::{db, writing, cv};
use htmx_blog::config::AppConfig;

#[launch]
async fn rocket() ->  _ {

    rocket::build()
        .mount("/", routes![index_admin, static_resources, js_resources, css_resources])
        .attach(db::stage())
        .attach(api::stage())
        .attach(writing::stage())
        .attach(cv::stage())
        .attach(AdHoc::config::<AppConfig>())
        .attach(Template::fairing())
}

#[get("/", rank=1)]
fn index_admin(user: Option<User>) -> Template { 

        match user {
            Some(_) =>  Template::render("index", context! { admin: true }),
            None =>  Template::render("index", context! { admin: false })
        }
}

#[get("/js/<path..>", rank = 2)]
async fn js_resources(path: PathBuf) -> Option<NamedFile> {
    let base_path = Path::new("./js/");
    let full_path = base_path.join(path);
    NamedFile::open(full_path).await.ok()
}

#[get("/styles.css", rank = 3)]
async fn css_resources() -> Option<NamedFile> {
    let full_path = Path::new("./static/styles.css");
    NamedFile::open(full_path).await.ok()
}

#[get("/<path..>", rank = 4)]
async fn static_resources(path: PathBuf) -> Option<NamedFile> {
    let base_path = Path::new("./static/");
    let full_path = base_path.join(path);
    NamedFile::open(full_path).await.ok()
}

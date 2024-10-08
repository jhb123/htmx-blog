use std::path::{PathBuf, Path};
use std::fs;

use htmx_blog::cv::{CV, Job};
use rocket::fairing::AdHoc;
use rocket::http::Status;
use rocket::{catch, catchers, get, launch, routes};
use rocket::fs::NamedFile;
use rocket_dyn_templates::{Template, context};
use rocket::serde::json::serde_json;

use htmx_blog::auth::api;
use htmx_blog::auth::api::User;
use htmx_blog::{db, writing};
use htmx_blog::config::AppConfig;




#[launch]
async fn rocket() ->  _ {

    rocket::build()
        .mount("/", routes![index_admin, static_resources, js_resources, css_resources, favicon])
        .register("/", catchers![not_found])
        .attach(db::stage())
        .attach(api::stage())
        .attach(writing::stage())
        // .attach(cv::stage())
        .attach(AdHoc::config::<AppConfig>())
        .attach(Template::fairing())
}

#[catch(404)]
fn not_found(req: &rocket::Request) -> (Status, Template) {
    (Status::NotFound, Template::render("not_found", context!{ }))
}

#[get("/", rank=1)]
fn index_admin(user: Option<User>) -> Template { 
    let path = "./static/cv.json";
    let data = fs::read_to_string(path).expect("Unable to read file");
    let cv_data: CV = serde_json::from_str(&data).unwrap();
    let job_data: Vec<Job> = cv_data.jobs.clone();
    match user {
        Some(_) =>  Template::render("index", context! { admin: true,  cv_data: &cv_data, job_data: job_data}),
        None =>  Template::render("index", context! { admin: false,  cv_data: &cv_data, job_data: job_data})
    }
}

#[get("/js/<path..>", rank = 2)]
async fn js_resources(path: PathBuf) -> (Status, Option<NamedFile>)  {
    let base_path = Path::new("./js/");
    let full_path = base_path.join(path);
    if full_path.is_dir() {
        return (Status::NotFound, None );
    }
    (Status::Ok , NamedFile::open(full_path).await.ok())
}

#[get("/styles.css", rank = 3)]
async fn css_resources() -> Option<NamedFile> {
    let full_path = Path::new("./static/styles.css");
    NamedFile::open(full_path).await.ok()
}


#[get("/favicon.png", rank = 3)]
async fn favicon() -> Option<NamedFile> {
    let full_path = Path::new("./static/favicon.png");
    NamedFile::open(full_path).await.ok()
}



#[get("/<path..>", rank = 4)]
async fn static_resources(path: PathBuf) -> Option<NamedFile> {
    let base_path = Path::new("./static/");
    let full_path = base_path.join(path);
    NamedFile::open(full_path).await.ok()
}

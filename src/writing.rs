use rocket::{fairing::AdHoc, routes, get, catchers, catch};
use rocket_dyn_templates::{Template, context};

use crate::auth::api::User;

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("blog-stage", |rocket| async {
        //rocket.attach(ArticlesDb::init())
        //    .attach(AdHoc::try_on_ignite("SQLx Migrations", run_migrations))
        rocket.mount("/writing", routes![main_blog_page_admin])
        .register("/writing", catchers![main_blog_page])

    })
}

#[catch(401)]
fn main_blog_page() -> Template { 
        Template::render("writing", context! { admin: false })
}

#[get("/", rank=1)]
fn main_blog_page_admin(_user: User) -> Template { 
        Template::render("writing", context! { admin: true })
}

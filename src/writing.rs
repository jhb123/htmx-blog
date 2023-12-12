use std::default;

use rocket::{fairing::AdHoc, routes, get, catchers, catch};
use rocket_dyn_templates::{Template, context};
use serde::Serialize;

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
    let dummy_data = vec![BlogData::default();100];
        Template::render("writing", context! { admin: false, blog_data: dummy_data })
}

#[get("/", rank=1)]
fn main_blog_page_admin(_user: User) -> Template { 
    let dummy_data = vec![BlogData::default();100];
    Template::render("writing", context! { admin: true, blog_data: dummy_data  })
}


#[derive(Clone, Serialize)]
struct BlogData {
    title: String,
    blurb: String,
    date: String,
    id: usize,
}

impl BlogData {
    fn default() -> BlogData{
        BlogData{
            title:"test Title".to_string(),
            blurb:"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.".to_string(),
            date:"01/12/2023".to_string(),
            id: 10000
        }
    }
}


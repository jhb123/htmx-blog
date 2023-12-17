use rocket::{
    State,
    fairing::AdHoc,
    form::Form,
    get,
    http::{Cookie, CookieJar, Status},
    post,
    request::{FromRequest, Outcome},
    response::Redirect,
    routes, catchers,
    time::OffsetDateTime,
    FromForm, Request, catch,
};
use rocket_db_pools::Connection;
use rocket_dyn_templates::{Template, context};
use serde::Deserialize;
use sqlx::Row;
use std::{
    borrow::Cow,
    fmt::{self},
    str::FromStr, convert::Infallible,
};
use rand::{thread_rng, Rng};

use crate::db::SiteDatabase;
use crate::config::AppConfig;
use crate::auth::util::validate_password;

type Result<T, E = rocket::response::Debug<sqlx::Error>> = std::result::Result<T, E>;

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Authentication-stage", |rocket| async {
        //rocket.attach(ArticlesDb::init())
        //    .attach(AdHoc::try_on_ignite("SQLx Migrations", run_migrations))
        rocket.mount("/auth", routes![login, logout, secured, logout_panel, login_panel])
        .register("/auth/panel", catchers![logout_catcher])
    })
}

#[derive(Deserialize, FromForm)]
struct Admin<'r> {
    pub r#password: &'r str,
}

// #[get("/toggle", rank=1)]
// fn admin_login_toggle() -> 

// #[get("/toggle", rank=2)]
// fn admin_login_toggle_logged_in(user: User) -> Template { 
//     Template::render("adminToggle", context! { title: "Hello, World", admin: true })
// }

#[get("/panel", rank=1)]
fn logout_panel(_user: User) -> Template { 
        Template::render("loginPanel", context! { admin: true })
}

#[catch(401)]
fn logout_catcher() -> Redirect { 
    Redirect::to("/auth/panel/login")
}

#[get("/panel/login", rank=1)]
fn login_panel() -> Template { 
    Template::render("loginPanel", context! { admin: false })
}


#[post("/login", data = "<admin>")]
async fn login(admin: Option<Form<Admin<'_>>>, cookies: &CookieJar<'_>, mut db: Connection<SiteDatabase>, app_config: &State<AppConfig>, url: HtmxCurrentUrl) -> Redirect {
    cookies.remove(Cookie::named("user_id"));

    match admin {
        Some(form_data) => {

            let entered_password = form_data.password;
            let admin_hash = &app_config.admin_hash;
            match validate_password(entered_password, &admin_hash[..]) {
                Ok(_) => {},
                Err(_) => return Redirect::to("/panel/login")//Template::render("loginPanel", context! { admin: false })
            }

            // gen
            let session_id: i32;
            {
                let mut rng = thread_rng();
                session_id = rng.gen();
            };
            
            let _ = sqlx::query("UPDATE users SET session=? WHERE id = 1")
                .bind(session_id)
                .execute(&mut *db).await.unwrap();

            let usr = User::Admin(session_id);
            let mut cookie = Cookie::new("user_id", &usr);
            let now = OffsetDateTime::now_utc();
            cookie.set_expires(now + rocket::time::Duration::hours(12));
            cookies.add_private(cookie);
            // usr.to_string()
            // Redirect::to("/")
            // let r = uri!(origin.path().to_string());
            Redirect::to(url.0)
            // Redirect::to(origin)
            // Template::render("index", context! { title: "Hello, World", admin: true })

        }
        None => Redirect::to(url.0) // Template::render("loginPanel", context! { admin: true })
    }
}

#[allow(unused)]
#[get("/secured")]
fn secured(user: User) -> Template { 
        Template::render("test", context! { info: "secured" })
        
}

#[derive(Deserialize, FromForm)]
struct Logout<'r> {
    pub r#url: &'r str,
}

#[post("/logout", data = "<logout_form>")]
fn logout(cookies: &CookieJar<'_>, logout_form: Form<Logout<'_>>) -> Redirect {
    cookies.remove(Cookie::named("user_id"));
    Redirect::to(logout_form.url.to_string()) 
}

#[derive(Debug)]
pub enum AuthError {
    MissingAuthCookie,
    InvalidUserPriviledge,
    PriviledgeExpired,
    CookieParseError,
    NoSessionId,
    InvalidSessionId,
    InvalidPassword,
}

impl std::error::Error for AuthError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AuthError::MissingAuthCookie => write!(f, "MissingAuthCookie"),
            AuthError::InvalidUserPriviledge => write!(f, "InvalidUserPriviledge"),
            AuthError::PriviledgeExpired => write!(f, "PriviledgeExpired"),
            AuthError::CookieParseError => write!(f, "CookieParseError"),
            AuthError::NoSessionId => write!(f, "NoSessionId"),
            AuthError::InvalidSessionId => write!(f, "InvalidSessionId"),
            AuthError::InvalidPassword => write!(f, "InvalidPassword"),
        }
    }
}

pub enum User {
    Admin(i32),
    SuperAdmin(i32),
    RegularUser,
}

impl From<&User> for Cow<'_, str> {
    fn from(user: &User) -> Self {
        Cow::Owned(user.to_string())
    }
}

impl fmt::Display for User {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            User::Admin(session_id) => write!(f, "Admin {session_id}"),
            User::SuperAdmin(session_id) => write!(f, "SuperAdmin {session_id}"),
            User::RegularUser => write!(f, "RegularUser"),
        }
    }
}

impl FromStr for User {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, anyhow::Error> {
        // let foo = s.split_once(" ");
        let (user_type, session_id) = s.split_once(" ").ok_or(AuthError::CookieParseError)?;
        let session_id = session_id.parse::<i32>()?;

        match user_type {
            "Admin" => Ok(User::Admin(session_id)),
            "SuperAdmin" => Ok(User::SuperAdmin(session_id)),
            _default => Err(AuthError::InvalidUserPriviledge.into()),
        }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = AuthError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<User, Self::Error> {
        // This was not obvious from the docs.
        let mut db = request.guard::<rocket_db_pools::Connection<SiteDatabase>>().await.succeeded().unwrap();

        match request
            .cookies()
            .get_private("user_id")
            .and_then(|cookie| cookie.value().parse::<User>().ok())
        {
            Some(user) => {
                match user {
                    User::Admin(session_id) => {
                        let res = sqlx::query("SELECT session FROM users WHERE id = 1")
                        .fetch_one(&mut *db).await.unwrap();

                        let server_session_id: Option::<i32> = res.get::<Option::<i32>,_>(0);
                        match  server_session_id {
                            Some(val) => {
                                if session_id == val {
                                    Outcome::Success(user)
                                } else {
                                    Outcome::Failure((Status::Unauthorized, AuthError::InvalidSessionId))
                                }
                            }
                            None => Outcome::Failure((Status::Unauthorized, AuthError::NoSessionId))
                        }
                    }
                    User::SuperAdmin(_) => Outcome::Success(user),
                    User::RegularUser => Outcome::Success(user),
                }
            }
            None => Outcome::Failure((Status::Unauthorized, AuthError::MissingAuthCookie)),
        }
    }
}

struct HtmxCurrentUrl(String);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for HtmxCurrentUrl {
    type Error = Infallible;


    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        println!("{:?}",req.headers());
        let url = req.headers().get_one("HX-Current-URL").unwrap();
        return  Outcome::Success(HtmxCurrentUrl(url.to_string()));
    }

}
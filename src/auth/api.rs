use rocket::{
    fairing::{AdHoc, self},
    form::Form,
    get,
    http::{Cookie, CookieJar, Status},
    post,
    request::{FromRequest, Outcome},
    routes,
    time::OffsetDateTime,
    FromForm, Request,
};
use rocket_db_pools::Connection;
use serde::Deserialize;
use std::{
    borrow::Cow,
    fmt::{self},
    str::FromStr,
};
use rand::{thread_rng, Rng};

use crate::db::{SiteDatabase, UserData};

type Result<T, E = rocket::response::Debug<sqlx::Error>> = std::result::Result<T, E>;

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Authentication-stage", |rocket| async {
        //rocket.attach(ArticlesDb::init())
        //    .attach(AdHoc::try_on_ignite("SQLx Migrations", run_migrations))
        rocket.mount("/auth", routes![login, logout, secured])
    })
}

#[derive(Deserialize, FromForm)]
struct Admin<'r> {
    pub r#password: &'r str,
}

#[post("/login", data = "<admin>")]
async fn login(admin: Option<Form<Admin<'_>>>, cookies: &CookieJar<'_>, mut db: Connection<SiteDatabase>) -> Result<String> {
    match admin {
        Some(form_data) => {
            // validate form_data i.e. check hashed password

            // gen
            let session_id: i32;
            {
                let mut rng = thread_rng();
                session_id = rng.gen();
            };
            
            let _ = sqlx::query("UPDATE users SET session=? WHERE id = 1")
                .bind(session_id)
                .execute(&mut *db).await?;

            let usr = User::Admin(session_id);
            let mut cookie = Cookie::new("user_id", &usr);
            let now = OffsetDateTime::now_utc();
            cookie.set_expires(now + rocket::time::Duration::hours(12));
            cookies.add_private(cookie);
            // usr.to_string()
            Ok("logged in".to_string())
        }
        None => Ok("invalid form data".to_string()),
    }
}

#[get("/secured")]
fn secured(user: User) -> String {
    user.to_string()
}

#[get("/logout")]
fn logout(cookies: &CookieJar<'_>) -> String {
    cookies.remove(Cookie::named("user_id"));
    "Logged out".to_string()
}

#[derive(Debug)]
pub enum AuthError {
    MissingAuthCookie,
    InvalidUserPriviledge,
    PriviledgeExpired,
    CookieParseError,
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
            User::Admin(time) => write!(f, "Admin {time}"),
            User::SuperAdmin(time) => write!(f, "SuperAdmin {time}"),
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
        match request
            .cookies()
            .get_private("user_id")
            .and_then(|cookie| cookie.value().parse::<User>().ok())
        {
            Some(user) => {
                match user {
                    User::Admin(session_id) => {
                        println!("{0}", session_id);
                        Outcome::Success(user)
                        // } else {
                        //     Outcome::Failure((Status::Unauthorized, AuthError::PriviledgeExpired))
                        // }
                    }
                    User::SuperAdmin(_) => Outcome::Success(user),
                    User::RegularUser => Outcome::Success(user),
                }
            }
            None => Outcome::Failure((Status::Unauthorized, AuthError::MissingAuthCookie)),
        }
    }
}
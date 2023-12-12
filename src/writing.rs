use std::{default, fs::{self, File}, io};

use kuchiki::{traits::TendrilSink, NodeRef};
use markdown::{to_html_with_options, Options, CompileOptions};
use rocket::{fairing::AdHoc, routes, get, catchers, catch, post, FromForm, fs::TempFile, form::Form, http::Status};
use rocket_db_pools::Connection;
use rocket_dyn_templates::{Template, context};
use serde::Serialize;
use sqlx::{QueryBuilder, Row};

use crate::{auth::api::User, db::SiteDatabase};

const ARTICLE_DIR: &str = "./articles";

type Result<T, E = rocket::response::Debug<sqlx::Error>> = std::result::Result<T, E>;

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("blog-stage", |rocket| async {
        //rocket.attach(ArticlesDb::init())
        //    .attach(AdHoc::try_on_ignite("SQLx Migrations", run_migrations))
        rocket.mount("/writing", routes![main_blog_page_admin, upload_form])
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

#[derive(FromForm)]
struct Upload<'r> {
    article_id: Option<i64>,
    title: Option<String>,
    title_image: Option<String>,
    blurb: Option<String>,
    files: Vec<TempFile<'r>>,
}

enum DatabaseErrors {
    ArticleIdNotFound,
    BadQuery(String),
}

#[post("/upload", data = "<upload>")]
async fn upload_form(user: User, mut upload: Form<Upload<'_>>, db: Connection<SiteDatabase>) -> (Status, String){ 

    
    let article_id = match upload.article_id {
        Some(x) => {
            // update database
            match update_article(&upload, db).await {
                Ok(_) => x.to_string(),
                Err(DatabaseErrors::ArticleIdNotFound) => return (Status::BadRequest, format!("No article with id {} exists",x)),
                Err(DatabaseErrors::BadQuery(msg)) =>  return (Status::InternalServerError, msg),
            }
            
        },
        None => {
            match create_article(&upload, db).await {
                Ok(primary_key) => primary_key.to_string(),
                Err(error) => return (Status::InternalServerError,error.0.to_string()),
            }
        },
    };

    let dir = format!("{ARTICLE_DIR}/{article_id}");

    if let Err(error) = fs::create_dir_all(&dir){
        return (Status::InternalServerError,error.to_string())
    }

    // Save each file that is included with the form. If its markdown, generate a html
    // file as well
    for file in upload.files.iter_mut(){
        if let Some(content_type) = file.content_type() {
            if content_type.is_markdown() {
                if let Err(error) = generate_article_html(&article_id, file){
                    return (Status::InternalServerError,error.to_string())
                }
            }
            if let Err(error) = save_article_item(&article_id, file).await {
                return (Status::InternalServerError,error.to_string())
            };
        } else {
            return (Status::BadRequest,"Missing content type for upload".to_string())
        }
    }
    (Status::Accepted,"uploaded".to_string())
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

async fn create_article(upload: &Form<Upload<'_>>, mut db: Connection<SiteDatabase>) -> Result<u64>{

    let query_result = sqlx::query("INSERT INTO articles 
    (is_published, visits, title, title_image, blurb) 
    VALUES (false, 0, ?, ?, ?)")
        .bind(&upload.title)
        .bind(&upload.title_image)
        .bind(&upload.blurb)
        .execute(&mut *db)
        .await?;

    Ok(query_result.last_insert_id())
 }

 async fn update_article(upload: &Form<Upload<'_>>, mut db: Connection<SiteDatabase>) -> Result<(), DatabaseErrors>{
    // let null_str = "Null".to_string();
    let title = upload.title.as_ref();
    let title_image = upload.title_image.as_ref();
    let blurb = upload.blurb.as_ref();
    let article_id = upload.article_id.unwrap();

    if title.is_none() && title_image.is_none() && blurb.is_none() {

        //sqlx::query("SELECT EXISTS (SELECT * FROM articles WHERE article_id=?) AS result");

        return match sqlx::query("SELECT EXISTS (SELECT * FROM articles WHERE article_id=?) AS result")
            .bind(&article_id)
            .fetch_one(&mut *db)
            .await {
                Ok(result) => {
                    if result.get::<i64,_>(0) == 0 {
                        return Err(DatabaseErrors::ArticleIdNotFound)
                    } else {
                        return Ok(())
                    }
                },
                Err(error) => Err(DatabaseErrors::BadQuery(error.to_string()))
            };
        
    }

    let mut query_builder = QueryBuilder::new("UPDATE articles SET ");

    // OK, this isn't the most elegant thing ever. I could have used Diesel for this
    // If I ever need to have this functionality somewhere else, I'll make it into
    // its own function.
    let mut enable_seperator = false;
    if let Some(title) = title {
        enable_seperator = true;
        query_builder.push("title =  ");
        query_builder.push_bind(title);
    }        
    if let Some(title_image) = title_image {
        if enable_seperator {
            query_builder.push(", ");
        } else { enable_seperator = true;};
        query_builder.push("title_image = ");
        query_builder.push_bind(title_image);
    }
    if let Some(blurb) = blurb {
        if enable_seperator {
            query_builder.push(", ");
        } // else { enable_seperator = true;};
        query_builder.push("blurb = ");
        query_builder.push_bind(blurb);
    }

    query_builder.push(" WHERE article_id = ");
    query_builder.push_bind(article_id);


    match query_builder.build().execute(&mut *db).await {
        Ok(_) => Ok(()),
        Err(error) => Err(DatabaseErrors::BadQuery(error.to_string()))
    }

}


async fn save_article_item( article_id: &String, file: &mut TempFile<'_>) -> io::Result<()> {
    let name = file.name().ok_or_else(|| io::Error::new(io::ErrorKind::Other, "File has no name"))?;
    let content_type = file
        .content_type()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "File has no content type"))?
        .to_owned();
    let ext = content_type.extension().ok_or_else(|| io::Error::new(io::ErrorKind::Other, "File has no extension"))?;  
    let full_name = [name, &ext.to_string()].join(".");
    let dir = format!("{ARTICLE_DIR}/{article_id}");

    file.persist_to( format!("{dir}/{full_name}")).await?;
    Ok(()) 
}

fn generate_article_html( guid: &String, file: &mut TempFile<'_>) -> Result<(), Box<dyn std::error::Error>> {
    
    // Read the markdown to a string
    let markdown_path = file.path().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Markdown file not found"))?;
    let markdown = fs::read_to_string(markdown_path)?;
    
    // Convert the markdown string to HTML string. The dangerous part here
    // is the markdown can have html in it, so I could put a malicous script
    // in my posts. The reason for dangerously parsing the markdown is so that
    // I can specify use <img> tags in the markdown. I guess I could add
    // a thing which removes <script> tags.
    let html = to_html_with_options(&markdown,
        &Options {
        compile: CompileOptions {
          allow_dangerous_html: true,
          ..CompileOptions::default()
        },
        ..Options::default()
    }).unwrap(); //this unwrap is safe according to the documentation 

    // Parse the html string to tree
    let document = kuchiki::parse_html().one(html);

    modify_dom_img_src(&document, &guid);
    
    // serialise the modified DOM to a html file
    let mut result = Vec::new();
    document.serialize(&mut result)?;
    let modified_html = String::from_utf8(result)?;
    let html_path = format!("{ARTICLE_DIR}/{guid}/generated.html");
    _ = File::create(&html_path)?;
    fs::write(html_path, modified_html)?;

    Ok(())   
}

fn modify_dom_img_src(document: &NodeRef, guid: &String){
    // this function is for parsing the <img src="..."> in a html document and replacing 
    // the src with valid urls
    if let Ok(img_nodes) = document.select("img"){
        for node in img_nodes {
            let mut attrs =  node.attributes.borrow_mut();
            match attrs.get_mut("src") {
                Some(src) => {
                    let trimmed = src.trim();
                    let new_src = match trimmed {
                        x if x.starts_with("https://") => x.to_string(),
                        x if x.starts_with("http://") => x.to_string(),
                        x if x.starts_with("./") => {
                            format!("/articles/{guid}/image/").to_owned() + &x[2..]
                        },
                        x => format!("/articles/{guid}/image/").to_owned() + x
                    };
                    src.replace_range(..,&new_src);
                }
                None => {}
            }        
        }
    }
}
use std::{default, fs::{self, File}, io};

use chrono::Utc;
use kuchiki::{traits::TendrilSink, NodeRef};
use markdown::{to_html_with_options, Options, CompileOptions};
use rocket::{fairing::AdHoc, routes, get, catchers, catch, post, FromForm, fs::TempFile, form::Form, http::Status};
use rocket_db_pools::Connection;
use rocket_dyn_templates::{Template, context};
use rocket::serde::{Serialize, Deserialize};
use sqlx::{QueryBuilder, Row, pool::PoolConnection, MySql};
use sqlx::types::chrono::DateTime;

use crate::{auth::api::User, db::SiteDatabase};

const WRITING_DIR: &str = "./writing";

type Result<T, E = rocket::response::Debug<sqlx::Error>> = std::result::Result<T, E>;

#[derive(FromForm)]
struct Upload<'r> {
    document_id: Option<i64>,
    title: Option<String>,
    blurb: Option<String>,
    files: Vec<TempFile<'r>>,
    tags: Option<String>
}

enum DatabaseErrors {
    ArticleIdNotFound,
    BadQuery(String),
}

#[derive(Debug, Clone, Deserialize, Serialize, sqlx::FromRow)]
#[serde(crate = "rocket::serde")]
struct DocumentMetaData {
    id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    creation_date: Option<DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    published_date: Option<DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_published: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    visits: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    // #[serde(skip_serializing_if = "Option::is_none")]
    // tags: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    blurb: Option<String>,
}


pub fn stage() -> AdHoc {
    AdHoc::on_ignite("blog-stage", |rocket| async {
        //rocket.attach(ArticlesDb::init())
        //    .attach(AdHoc::try_on_ignite("SQLx Migrations", run_migrations))
        rocket.mount("/writing", routes![main_blog_page_admin, upload_form])
        // .register("/writing", catchers![main_blog_page])

    })
}


#[get("/", rank=1)]
async fn main_blog_page_admin(user: Option<User>, mut db: Connection<SiteDatabase>) -> Template { 
    let data  = sqlx::query_as::<_, DocumentMetaData>("SELECT * FROM writing").fetch_all(&mut *db).await.unwrap_or(vec![]);

    match user {
        Some(_) => Template::render("writing",context!{admin:true,blog_data:data}),
        None => Template::render("writing",context!{admin:false, blog_data: data}), 
    }

}

#[post("/upload", data = "<upload>")]
async fn upload_form(user: User, mut upload: Form<Upload<'_>>, mut db: Connection<SiteDatabase>) -> (Status, Template){ 

    

    let article_id = match upload.document_id {
        Some(x) => {
            // update database
            match update_article(&upload, &mut *db).await {
                Ok(_) => x.to_string(),
                Err(DatabaseErrors::ArticleIdNotFound) => {
                    let error_message = Template::render(
                        "test", context! { info: format!("No article with id {} exists",x) }
                    );
                    return (Status::BadRequest, error_message)
                },
                Err(DatabaseErrors::BadQuery(msg)) => {
                    let error_message = Template::render(
                        "test", context! { info: msg }
                    );   
                    return (Status::InternalServerError, error_message)},
            }            
        },
        None => {
            println!("creating new article");
            match create_article(&upload, &mut *db).await {
                Ok(primary_key) => primary_key.to_string(),
                Err(error) => {
                    let error_message = Template::render(
                        "test", context! { info: error.0.to_string() }
                    );   
                    return (Status::InternalServerError, error_message)
                },
            }
        },
    };

    let dir = format!("{WRITING_DIR}/{article_id}");

    if let Err(error) = fs::create_dir_all(&dir){
        let error_message = Template::render(
            "test", context! { info: error.to_string() }
        );   
        return (Status::InternalServerError, error_message)
    }

    // Save each file that is included with the form. If its markdown, generate a html
    // file as well
    for file in upload.files.iter_mut(){
        if let Some(content_type) = file.content_type() {
            if content_type.is_markdown() {
                if let Err(error) = generate_article_html(&article_id, file){
                    let error_message = Template::render(
                        "test", context! { info: error.to_string() }
                    );   
                    return (Status::InternalServerError, error_message)
                }
            }
            if let Err(error) = save_article_item(&article_id, file).await {
                let error_message = Template::render(
                    "test", context! { info: error.to_string() }
                );   
                return (Status::InternalServerError, error_message)
            };
        } else {
            let error_message = Template::render(
                "test", context! { info: "Missing content type for upload".to_string() }
            );   
            return (Status::BadRequest, error_message)
        }
    }
    

    // let res = get_document_list(db).await;
    let res  = sqlx::query_as::<_, DocumentMetaData>("SELECT * FROM writing").fetch_all(&mut *db).await;

    match res {
        Ok(documents) => {
            let template = Template::render("document_list", context! { blog_data: documents });
            return (Status::Accepted,template)
        },
        Err(err) => {
            let error_message = Template::render(
                "test", context! { info: format!("{:?}",err) }
            );   
            return (Status::InternalServerError, error_message)
        }
    }

}




// #[get("/list")]
// async fn list(mut db: Connection<SiteDatabase>) -> Result<Template> { 
//     let document_data = get_article_list(db).await?;
//     Ok(Template::render("writing", context! { document_data: document_data  }))
// }

async fn get_document_list(db: &mut PoolConnection<MySql>) -> Result<Template>{
    let res: Vec<DocumentMetaData>  = sqlx::query_as::<_, DocumentMetaData>("SELECT * FROM writing").fetch_all(db).await?;

    // let res2: Vec<DocumentMetaData>  = sqlx::query_as::<_, DocumentMetaData>("SELECT * FROM writing").fetch_all(&mut *db).await?;

    Ok(Template::render("document_list", context! { documents: res }))
}

// fn render_document_list(data: Vec<DocumentMetaData>) -> Template {
// }

async fn create_article(upload: &Form<Upload<'_>>, db: &mut PoolConnection<MySql>) -> Result<u64>{

    let query_result = sqlx::query("INSERT INTO writing 
    (is_published, visits, title, blurb) 
    VALUES (false, 0, ?, ?)")
        .bind(&upload.title)
        .bind(&upload.blurb)
        .execute(db)
        .await?;

    Ok(query_result.last_insert_id())
 }

 async fn update_article(upload: &Form<Upload<'_>>,  db: &mut PoolConnection<MySql>) -> Result<(), DatabaseErrors>{
    // let null_str = "Null".to_string();
    let title = upload.title.as_ref();
    let blurb = upload.blurb.as_ref();
    let article_id = upload.document_id.unwrap();

    if title.is_none() && blurb.is_none() {

        //sqlx::query("SELECT EXISTS (SELECT * FROM articles WHERE article_id=?) AS result");

        return match sqlx::query("SELECT EXISTS (SELECT * FROM writing WHERE id=?) AS result")
            .bind(&article_id)
            .fetch_one(db)
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

    let mut query_builder = QueryBuilder::new("UPDATE writing SET ");

    // OK, this isn't the most elegant thing ever. I could have used Diesel for this
    // If I ever need to have this functionality somewhere else, I'll make it into
    // its own function.
    let mut enable_seperator = false;
    if let Some(title) = title {
        enable_seperator = true;
        query_builder.push("title =  ");
        query_builder.push_bind(title);
    }        
    if let Some(blurb) = blurb {
        if enable_seperator {
            query_builder.push(", ");
        } // else { enable_seperator = true;};
        query_builder.push("blurb = ");
        query_builder.push_bind(blurb);
    }

    query_builder.push(" WHERE id = ");
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
    let dir = format!("{WRITING_DIR}/{article_id}");

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
    let html_path = format!("{WRITING_DIR}/{guid}/generated.html");
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
                            format!("/writing/{guid}/image/").to_owned() + &x[2..]
                        },
                        x => format!("/writing/{guid}/image/").to_owned() + x
                    };
                    src.replace_range(..,&new_src);
                }
                None => {}
            }        
        }
    }
}

impl DocumentMetaData {
    fn default() -> DocumentMetaData{
        DocumentMetaData{
            title: Some("test Title".to_string()),
            blurb:Some("Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.".to_string()),
            id: 10000,
            creation_date: Some(Utc::now()),
            published_date: Some(Utc::now()),
            is_published: Some(true),
            visits: Some(100),
        }
    }
}
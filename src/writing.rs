use std::{ fs::{self, File}, io, fmt};
use kuchiki::{traits::TendrilSink, NodeRef};
use markdown::{to_html_with_options, Options, CompileOptions};
use rocket::{fairing::AdHoc, routes, get, post, FromForm, fs::{TempFile, NamedFile}, form::Form, http::Status, response::status::NotFound, delete, State};

use rocket_db_pools::Connection;
use rocket_dyn_templates::{Template, context};
use rocket::serde::{Serialize, Deserialize};
use sqlx::{QueryBuilder, Row, pool::PoolConnection, Sqlite};
use sqlx::types::chrono::DateTime;

use crate::{auth::api::User, db::SiteDatabase, config::AppConfig};


pub fn stage() -> AdHoc {
    AdHoc::on_ignite("blog-stage", |rocket| async {
        rocket.mount("/writing", routes![main_blog_page_admin, upload_form,get_article,get_image,publish,delete_stuff, search, tags])
    })
}


//const WRITING_DIR: &str = "./writing";

type Result<T, E = rocket::response::Debug<sqlx::Error>> = std::result::Result<T, E>;

#[allow(unused)]
#[derive(FromForm)]
struct Upload<'r> {
    document_id: Option<i64>,
    title: String,
    blurb: String,
    files: Vec<TempFile<'r>>,
    tag: String
}


trait RenderErrorTemplate {
    fn to_template(&self) -> Template;
}

impl RenderErrorTemplate for DatabaseErrors {
    fn to_template(&self) -> Template {
        Template::render("test", context! { info: format!("{}",self) })
    }
} 

impl RenderErrorTemplate for std::io::Error {
    fn to_template(&self) -> Template {
        Template::render("test", context! { info: format!("{}",self) })
    }
}

impl RenderErrorTemplate for dyn std::error::Error {
    fn to_template(&self) -> Template {
        Template::render("test", context! { info: format!("{}",self) })
    }
} 

impl RenderErrorTemplate for sqlx::Error {
    fn to_template(&self) -> Template {
        Template::render("test", context! { info: format!("{}",self) })
    }
} 


#[derive(Debug)]
enum DatabaseErrors {
    IdNotFound(String),
    BadQuery(String),
    SQLx(String)
}

impl From<sqlx::Error> for DatabaseErrors {
    fn from(error: sqlx::Error) -> Self {
        DatabaseErrors::SQLx(format!("Database error: {}", error))
        }
    }


impl fmt::Display for DatabaseErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DatabaseErrors::IdNotFound(message) => write!(f, "ID not found: {}", message),
            DatabaseErrors::BadQuery(message) => write!(f, "Bad query: {}", message),
            DatabaseErrors::SQLx(message) => write!(f, "SQLx error: {}", message),
        }
    }
}



#[derive(Debug, Clone, Deserialize, Serialize, sqlx::FromRow)]
#[serde(crate = "rocket::serde")]
struct DocumentMetaData {
    id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    creation_date: Option<DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    published_date: Option<DateTime<chrono::Utc>>,
    is_published: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    visits: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    blurb: Option<String>,
}


#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct FormattedDocumentMetaData {
    id: String,
    creation_date: Option<String>,
    published_date: Option<String>,
    is_published: i64,
    visits: Option<String>,
    title: Option<String>,
    // #[serde(skip_serializing_if = "Option::is_none")]
    tag: Option<String>,
    blurb: Option<String>,
}

impl DocumentMetaData {

    fn display(&self) -> FormattedDocumentMetaData {
        FormattedDocumentMetaData {
            id: self.id.to_string(),
            creation_date: self.creation_date.and_then(|f| Some(f.format("%d/%m/%Y %H:%M").to_string())),
            published_date: self.published_date.and_then(|f| Some(f.format("%A, %d %B %Y").to_string())),
            is_published: self.is_published,
            visits: self.visits.and_then(|f| Some(f.to_string())),
            title: self.title.to_owned(),
            // #[serde(skip_serializing_if = "Option::is_none")]
            tag: self.tag.to_owned(),
            blurb: self.blurb.to_owned(),
        }
    }

}



#[get("/", rank=1)]
async fn main_blog_page_admin(user: Option<User>, mut db: Connection<SiteDatabase>) -> Template { 
    let data  = sqlx::query_as::<_, DocumentMetaData>("SELECT * FROM writing ORDER BY id DESC").fetch_all(&mut *db).await.unwrap_or(vec![]);

    println!("{:?}", user);
    let filtered_data: Vec<FormattedDocumentMetaData> = match user {
        Some(_) => data.into_iter().map(|item| item.display()).collect(),
        None => data.into_iter().filter(|item| item.is_published == 1 ).map(|item| item.display()).collect()
    };

    let tag_data  = sqlx::query("SELECT tag FROM writing ORDER BY tag ASC").fetch_all(&mut *db).await;

    let tags: Vec<String> = match tag_data {
        Ok(row) => row.iter().filter_map(|x| x.try_get("tag").ok()).collect(),
        Err(e) => return e.to_template(),
    };

    match user {
        Some(_) => Template::render("writing",context!{admin:true,blog_data: filtered_data, tags_expanded: false, tags: tags}),
        None => Template::render("writing",context!{admin:false, blog_data: filtered_data, tags_expanded: false, tags: tags}), 
    }

}

#[get("/tags?<open>")]
async fn tags(open: bool, mut db: Connection<SiteDatabase>) -> Template {

    let tag_data  = sqlx::query("SELECT tag FROM writing ORDER BY tag ASC").fetch_all(&mut *db).await;

    let tags: Vec<String> = match tag_data {
        Ok(row) => row.iter().filter_map(|x| x.try_get("tag").ok()).collect(),
        Err(e) => return e.to_template(),
    };

    return Template::render("tag_tab", context!{tags_expanded: open, tags: tags})
}

#[get("/search?<title>&<tag>")]
async fn search(user: Option<User>, mut db: Connection<SiteDatabase>, title: Option<String>, tag: Option<String>) -> Template { 

    dbg!(&title);
    dbg!(&tag);

    let data = match (tag,title) {
        (None,None) => {
            sqlx::query_as::<_, DocumentMetaData>("SELECT * FROM writing ORDER BY id DESC")
                .fetch_all(&mut *db).await.unwrap_or(vec![])
        },
        (Some(tag),None) => {
            sqlx::query_as::<_, DocumentMetaData>("SELECT * FROM writing WHERE tag=? ORDER BY id DESC")
                .bind(tag)
                .fetch_all(&mut *db).await.unwrap_or(vec![])
        },
        (None,Some(title)) => {
            sqlx::query_as::<_, DocumentMetaData>("SELECT * FROM writing WHERE title LIKE ? ORDER BY id DESC")
                .bind(title)
                .fetch_all(&mut *db).await.unwrap_or(vec![])
        },
        (Some(title),Some(tag)) => {
            sqlx::query_as::<_, DocumentMetaData>("SELECT * FROM writing WHERE title LIKE ? and tag=? ORDER BY id DESC")
                .bind(title)
                .bind(tag)
                .fetch_all(&mut *db).await.unwrap_or(vec![])
        },

    };

    // let data  = 
    // sqlx::query_as::<_, DocumentMetaData>("SELECT * FROM writing WHERE title LIKE ? ORDER BY id DESC")
    //     .bind(title)
    //     .bind(tag)
    //     .fetch_all(&mut *db).await.unwrap_or(vec![]);

    println!("number of results: {}", data.len());

        let filtered_data: Vec<FormattedDocumentMetaData> = match user {
            Some(_) => data.into_iter().map(|item| item.display()).collect(),
            None => data.into_iter().filter(|item| item.is_published == 1 ).map(|item| item.display()).collect()
        };
    
        match user {
            Some(_) => Template::render("document_list",context!{admin:true,blog_data: filtered_data}),
            None => Template::render("document_list",context!{admin:false, blog_data: filtered_data}), 
        }
}



#[post("/upload", data = "<upload>")]
async fn upload_form(_user: User, mut upload: Form<Upload<'_>>, mut db: Connection<SiteDatabase>, app_config: &State<AppConfig>) -> (Status, Template){ 
 

    let res_document_id = match upload.document_id {
        Some(x) =>  update_article(&upload, &mut *db).await.map(|_| x),
        None => create_article(&upload, &mut *db).await
    };
    
    let document_id = match res_document_id.map_err(|db_error| {
        match db_error {
            DatabaseErrors::IdNotFound(_) => (Status::BadRequest, db_error.to_template()),
            DatabaseErrors::BadQuery(_) => (Status::BadRequest, db_error.to_template()),
            DatabaseErrors::SQLx(_) => (Status::InternalServerError, db_error.to_template()),
        }

    }) {
        Ok(value) => value.to_string(),
        Err((status, template)) => return (status, template),
    };

    //let base_dir = app_config.writing_dir;
    let dir = format!("{0}/{1}",app_config.writing_dir, document_id);

    if let Err(error) = fs::create_dir_all(&dir){
        return (Status::InternalServerError, error.to_template())
    }

    // Save each file that is included with the form. If its markdown, generate a html
    // file as well
    for file in upload.files.iter_mut(){
        if let Some(content_type) = file.content_type() {
            if content_type.is_markdown() {
                if let Err(error) = generate_article_html(&document_id, file, &app_config.writing_dir){
                    return (Status::InternalServerError, error.to_template())
                }
            }
            if let Err(error) = save_article_item(&document_id, file, &app_config.writing_dir).await {
                return (Status::InternalServerError, error.to_template())
            };
        } else {
            let error_message = Template::render(
                "test", context! { info: "Missing content type for upload".to_string() }
            );   
            return (Status::BadRequest, error_message)
        }
    }
    

    // let res = get_document_list(db).await;
    let res  = sqlx::query_as::<_, DocumentMetaData>("SELECT * FROM writing ORDER BY id DESC").fetch_all(&mut *db)
        .await
        .map_err(|e| DatabaseErrors::SQLx(e.to_string()));

    match res {
        Ok(documents) => {
            let template = Template::render("document_list", context! { admin: true, blog_data: documents });
            return (Status::Accepted,template)
        },
        Err(err) => {
            return (Status::InternalServerError, err.to_template())
        }
    }

}


#[get("/<document_id>")]
async fn get_article(document_id: &str, mut db: Connection<SiteDatabase>, app_config: &State<AppConfig>) -> (Status, Template) { 

    let res = sqlx::query("UPDATE writing SET visits = visits+1 WHERE id = ?")
    .bind(document_id)
    .execute(&mut *db)
    .await;

    if res.is_err() {
        return (Status::InternalServerError, res.unwrap_err().to_template())
    }
    let path = format!("{0}/{1}/generated.html",app_config.writing_dir, document_id);

    //let path = format!("{WRITING_DIR}/{document_id}/generated.html");
    let res  = sqlx::query_as::<_, DocumentMetaData>("SELECT * FROM writing WHERE id=?")
        .bind(document_id)
        .fetch_one(&mut *db)
        .await;


    let meta_data = match res {
        Ok(meta_data) => meta_data.display(),
        Err(err) => return (Status::NotFound, err.to_template()),
    };

    let document_title = meta_data.title.unwrap_or(format!("Document {}", meta_data.id));

    let published_date = meta_data.published_date.unwrap_or("-".to_string());

    match fs::read_to_string(path) {
        Ok(html) => {
            (Status::Ok, Template::render("document", context! {raw_data: html, document_title: document_title,published_date: published_date }))
        },
        Err(err) => (Status::InternalServerError, err.to_template())

    }
}

#[get("/<document_id>/image/<name>", rank=2)]
async fn get_image(document_id: &str, name: &str, app_config: &State<AppConfig>) -> Result<NamedFile, NotFound<String>> {
    let path = format!("{0}/{1}/{2}",app_config.writing_dir, document_id, name);
    NamedFile::open(&path).await.map_err(|e| NotFound(e.to_string()))
}

#[post("/<document_id>/publish/<publish>")]
async fn publish(_user: User, mut db: Connection<SiteDatabase>, document_id: i64, publish: bool) -> Template { 
    println!("is published {}", publish);
    let time  = if publish { Some(chrono::Utc::now())} else {None};

    let res = sqlx::query("UPDATE writing SET is_published = ?, published_date = ? WHERE id = ?")
        .bind(publish)
        .bind(time)
        .bind(document_id)
        .execute(&mut *db)
        .await;

    match res {
        Ok(_) => {
            let data  = sqlx::query_as::<_, DocumentMetaData>("SELECT * FROM writing WHERE id=?").bind(document_id).fetch_one(&mut *db).await.unwrap();

            // match data.published_date {
            //     Some(time) => format!("{}", date_time.format("%d/%m/%Y %H:%M")),
            //     None => todo!(),
            // }
            

            Template::render("document_list_item", context! {admin: true, data: data.display() })
        },
        Err(err) => err.to_template(),
    }
        

    // Ok(format!("set article {0} to published={1}",document_id, is_published))
}

#[allow(unused)]
#[delete("/<document_id>/delete")]
async fn delete_stuff(user: User, mut db: Connection<SiteDatabase>, document_id: i64, app_config: &State<AppConfig>)-> Result<()> {
    let _ = sqlx::query("DELETE FROM writing WHERE id = ?")
    .bind(document_id)
    .execute(&mut *db)
    .await?;

    let dir = format!("{0}/{1}",app_config.writing_dir, document_id);

    let _ = fs::remove_dir_all(dir).map_err(|e| NotFound(e.to_string()));
    Ok(())
}

async fn create_article(upload: &Form<Upload<'_>>, db: &mut PoolConnection<Sqlite>) -> Result<i64, DatabaseErrors>{

    let query_result = sqlx::query("INSERT INTO writing 
    (is_published, visits, title, blurb, tag) 
    VALUES (false, 0, ?, ?, ?)")
        .bind(&upload.title)
        .bind(&upload.blurb)
        .bind(&upload.tag)
        .execute(db)
        .await?;

    println!("last insert id {}",query_result.last_insert_rowid());

    Ok(query_result.last_insert_rowid())
 }

 async fn update_article(upload: &Form<Upload<'_>>,  db: &mut PoolConnection<Sqlite>) -> Result<(), DatabaseErrors>{
    // let null_str = "Null".to_string();
    let title = if &upload.title != "" {Some(&upload.title)} else {None};//.
    let blurb = if &upload.blurb != "" {Some(&upload.blurb)} else {None};//.
    let tag = if &upload.tag != "" {Some(&upload.tag)} else {None};//.
    let document_id = upload.document_id.unwrap();

    if title.is_none() && blurb.is_none() && tag.is_none() {

        //sqlx::query("SELECT EXISTS (SELECT * FROM articles WHERE document_id=?) AS result");

        return match sqlx::query("SELECT EXISTS (SELECT * FROM writing WHERE id=?) AS result")
            .bind(&document_id)
            .fetch_one(db)
            .await {
                Ok(result) => {
                    if result.get::<i64,_>(0) == 0 {
                        return Err(DatabaseErrors::IdNotFound(format!("No document found with ID {}", document_id)))
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
    if let Some(tag) = tag {
        if enable_seperator {
            query_builder.push(", ");
        } // else { enable_seperator = true;};
        query_builder.push("tag = ");
        query_builder.push_bind(tag);
    }

    query_builder.push(" WHERE id = ");
    query_builder.push_bind(document_id);


    match query_builder.build().execute(&mut *db).await {
        Ok(_) => Ok(()),
        Err(error) => Err(DatabaseErrors::BadQuery(error.to_string()))
    }
    

}


async fn save_article_item( document_id: &String, file: &mut TempFile<'_>, base_dir: &String) -> io::Result<()> {
    let name = file.name().ok_or_else(|| io::Error::new(io::ErrorKind::Other, "File has no name"))?;
    let content_type = file
        .content_type()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "File has no content type"))?
        .to_owned();
    let ext = content_type.extension().ok_or_else(|| io::Error::new(io::ErrorKind::Other, "File has no extension"))?;  
    let full_name = [name, &ext.to_string()].join(".");
    let dir = format!("{base_dir}/{document_id}");

    file.copy_to( format!("{dir}/{full_name}")).await?;
    println!("saving to {dir}/{full_name}");
    Ok(()) 
}

fn generate_article_html( guid: &String, file: &mut TempFile<'_>, base_dir: &String) -> Result<(), Box<dyn std::error::Error>> {
    
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
          allow_dangerous_protocol: true,
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
    // the tree gets these tags, which we don't need.
    let i1 =  "<html><head></head><body>".len();
    let i2 =  modified_html.len() - "</body></html>".len();

    let html_path = format!("{base_dir}/{guid}/generated.html");
    _ = File::create(&html_path)?;
    fs::write(html_path, modified_html[i1..i2].to_string())?;

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

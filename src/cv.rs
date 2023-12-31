use std::collections::HashMap;
use serde::{Serialize, Deserialize};


// pub fn stage() -> AdHoc {
//     AdHoc::on_ignite("blog-stage", |rocket| async {
//         rocket.mount("/cv", routes![cv])
//     })
// }


// #[get("/", rank=1)]
// fn cv() -> Template { 

//     let path = "./static/cv.json";
//     let data = fs::read_to_string(path).expect("Unable to read file");
//     let cv_data: CV = serde_json::from_str(&data).unwrap();
//     Template::render("cv_main", context! { cv_data: &cv_data, job_data: &cv_data.jobs })
// }


#[derive(Serialize, Deserialize, Debug)]
pub struct CV {
    intro: String,
    skills: Skills,
    #[serde(rename = "programming projects")]
    programming_projects: HashMap<String, Project>,
    pub jobs: Vec<Job>,
    education: HashMap<String, Qualification>,
    training: HashMap<String, Course>,
    interests: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Skills {
    #[serde(rename = "programming languages")]
    programming_languages: HashMap<String, Vec<String>>,
    UI: Vec<String>,
    #[serde(rename = "libraries and frameworks")]
    libs_frames: Vec<String>,
    #[serde(rename = "programming tools")]
    tools: Vec<String>,
    #[serde(rename = "technical skills")]
    hard: Vec<String>,
    #[serde(rename = "soft skills")]
    soft: Vec<String>,
    cloud: Vec<String> 
}


#[derive(Serialize, Deserialize, Debug)]
struct Project {
    name: String,
    blurb: String,
    links: Vec<String>,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Job {
    company: String,
    roles: Vec<JobRole>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct JobRole {
    title: String,
    blurb: Option<String>,
    dates: String,
    highlights: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Qualification {
    university: String,
    degree: String,
    dates: String,
    grade: String,
    highlights: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Course {
    title: String,
    provider: String,
    blurb: String,
}



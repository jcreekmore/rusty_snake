use common_failures::prelude::*;
use common_failures::quick_main;
use dirs::home_dir;
use reqwest;
use reqwest::header::{self, HeaderName, HeaderValue};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::env::var_os;
use std::fs::File;
use std::process::exit;
use structopt::StructOpt;

const CACHE: &str = ".mr.cache";

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Group {
    id: usize,
    name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Author {
    username: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Project {
    name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct MergeRequests {
    project_id: usize,
    title: String,
    web_url: String,
    author: Author,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ExpandedMergeRequests {
    project_name: String,
    group_name: String,
    title: String,
    web_url: String,
    author: Author,
}

fn inc(session: reqwest::Client) -> Result<()> {
    let gitlab = var_os("GITLAB_URL")
        .map(|val| val.to_string_lossy().into_owned())
        .unwrap_or_else(|| "https://gitlab.com".into());

    let groups: Vec<Group> = session
        .get(&format!("{}/api/v4/groups", gitlab))
        .send()?
        .json()?;

    let mut merge_requests = vec![];

    for group in groups {
        let grp_merge_requests: Vec<MergeRequests> = session
            .get(&format!(
                "{}/api/v4/groups/{}/merge_requests",
                gitlab, group.id
            ))
            .query(&[("state", "opened")])
            .send()?
            .json()?;

        for mr in grp_merge_requests {
            merge_requests.push((group.clone(), mr));
        }
    }

    let mut cache: Vec<ExpandedMergeRequests> = vec![];
    let mut project_names: HashMap<usize, String> = HashMap::new();

    for (group, mr) in merge_requests {
        if !project_names.contains_key(&mr.project_id) {
            let project: Project = session
                .get(&format!("{}/api/v4/projects/{}", gitlab, mr.project_id))
                .send()?
                .json()?;
            project_names.insert(mr.project_id, project.name);
        }

        let obj = ExpandedMergeRequests {
            project_name: project_names[&mr.project_id].clone(),
            group_name: group.name,
            title: mr.title,
            web_url: mr.web_url,
            author: mr.author,
        };

        cache.push(obj);
    }

    let cachefilename = home_dir().unwrap().join(CACHE);
    let fp = File::create(&cachefilename).io_write_context(&cachefilename)?;
    serde_json::to_writer_pretty(fp, &cache)?;

    Ok(())
}

fn show(idx: Option<usize>) -> Result<()> {
    let cachefilename = home_dir().unwrap().join(CACHE);
    let fp = File::open(&cachefilename).io_read_context(&cachefilename)?;
    let merge_requests: Vec<ExpandedMergeRequests> = serde_json::from_reader(fp)?;

    match idx {
        None => {
            for (idx, mr) in merge_requests.iter().enumerate() {
                println!("{:3}: [{}/{}] {}",
                         idx, mr.group_name, mr.project_name, mr.title)
            }
        }
        Some(idx) => {
            if let Some(mr) = merge_requests.get(idx) {
                println!(
                    "[{}/{}] {} - @{}",
                    mr.group_name, mr.project_name, mr.title,
                    mr.author.username
                );
                println!("     {}", mr.web_url);
            } else {
                eprintln!(
                    "Invalid merge request: {} is larger than {}",
                    idx,
                    merge_requests.len() - 1
                );
            }
        }
    }

    Ok(())
}

#[derive(Debug, StructOpt)]
#[structopt(name = "Merge Requests")]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
#[structopt(raw(setting = "structopt::clap::AppSettings::SubcommandRequiredElseHelp"))]
enum Config {
    /// incorporate merge requests
    #[structopt(name = "inc")]
    Inc,
    /// show incorporated merge requests
    #[structopt(name = "show")]
    Show {
        /// which merge request index
        idx: Option<usize>,
    }
}

fn run() -> Result<()> {
    let private_token = match var_os("GITLAB_PRIVATE_TOKEN") {
        Some(token) => token.to_string_lossy().into_owned(),
        None => {
            eprintln!("GITLAB_PRIVATE_TOKEN was not set");
            exit(1)
        }
    };

    let mut headers = header::HeaderMap::new();
    headers.insert(
        HeaderName::from_static("private-token"),
        HeaderValue::from_str(&private_token)?,
    );
    let session = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    let config = Config::from_args();

    match config {
        Config::Inc => inc(session),
        Config::Show { idx } => show(idx),
    }
}

quick_main!(run);

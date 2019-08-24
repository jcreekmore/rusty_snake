use clap::{App, Arg, SubCommand};
use dirs::home_dir;
use reqwest;
use reqwest::header::{self, HeaderName, HeaderValue};
use serde_json::Value;
use std::collections::HashMap;
use std::env::var_os;
use std::fs::File;
use std::process::exit;

const CACHE: &str = ".mr.cache";

fn inc(session: reqwest::Client) {
    let gitlab = var_os("GITLAB_URL")
        .map(|val| val.to_string_lossy().into_owned())
        .unwrap_or_else(|| "https://gitlab.com".into());

    let groups: Vec<Value> = session
        .get(&format!("{}/api/v4/groups", gitlab))
        .send()
        .unwrap()
        .json()
        .unwrap();

    let mut merge_requests = vec![];

    for group in groups {
        let group_id = group.get("id").unwrap().as_u64().unwrap();

        let grp_merge_requests: Vec<Value> = session
            .get(&format!(
                "{}/api/v4/groups/{}/merge_requests",
                gitlab, group_id
            ))
            .query(&[("state", "opened")])
            .send()
            .unwrap()
            .json()
            .unwrap();

        for mr in grp_merge_requests {
            merge_requests.push((group.clone(), mr));
        }
    }

    let mut cache: Vec<Value> = vec![];
    let mut project_names: HashMap<u64, String> = HashMap::new();

    for (group, mut mr) in merge_requests {
        let proj_id = mr.get("project_id").unwrap().as_u64().unwrap();
        let group_name = group.get("name").unwrap().as_str().unwrap();
        if !project_names.contains_key(&proj_id) {
            let project: Value = session
                .get(&format!("{}/api/v4/projects/{}", gitlab, proj_id))
                .send()
                .unwrap()
                .json()
                .unwrap();
            let name = project.get("name").unwrap().as_str().unwrap().to_string();
            project_names.insert(proj_id, name);
        }
        let obj = mr.as_object_mut().unwrap();
        obj.insert(
            "project_name".into(),
            Value::String(project_names[&proj_id].clone()),
        );
        obj.insert("group_name".into(), Value::String(group_name.to_string()));
        cache.push(mr);
    }

    let cachefilename = home_dir().unwrap().join(CACHE);
    let fp = File::create(cachefilename).unwrap();
    serde_json::to_writer_pretty(fp, &cache).unwrap();
}

fn show(idx: Option<usize>) {
    let cachefilename = home_dir().unwrap().join(CACHE);
    let fp = File::open(cachefilename).unwrap();
    let merge_requests: Vec<Value> = serde_json::from_reader(fp).unwrap();

    match idx {
        None => {
            for (idx, mr) in merge_requests.iter().enumerate() {
                let group_name = mr.get("group_name").unwrap().as_str().unwrap();
                let project_name = mr.get("project_name").unwrap().as_str().unwrap();
                let title = mr.get("title").unwrap().as_str().unwrap();
                println!("{:3}: [{}/{}] {}", idx, group_name, project_name, title)
            }
        }
        Some(idx) => {
            if let Some(mr) = merge_requests.get(idx) {
                let username = mr
                    .get("author")
                    .unwrap()
                    .as_object()
                    .unwrap()
                    .get("username")
                    .unwrap()
                    .as_str()
                    .unwrap();
                let url = mr.get("web_url").unwrap().as_str().unwrap();

                let group_name = mr.get("group_name").unwrap().as_str().unwrap();
                let project_name = mr.get("project_name").unwrap().as_str().unwrap();
                let title = mr.get("title").unwrap().as_str().unwrap();
                println!(
                    "[{}/{}] {} - @{}",
                    group_name, project_name, title, username
                );
                println!("     {}", url);
            } else {
                eprintln!(
                    "Invalid merge request: {} is larger than {}",
                    idx,
                    merge_requests.len() - 1
                );
            }
        }
    }
}

fn main() {
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
        HeaderValue::from_str(&private_token).unwrap(),
    );
    let session = reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap();

    let mut app = App::new("Merge Requests")
        .subcommand(SubCommand::with_name("inc"))
        .subcommand(SubCommand::with_name("show").arg(Arg::with_name("idx")));
    let matches = app.clone().get_matches();

    if matches.subcommand_matches("inc").is_some() {
        inc(session);
    } else if let Some(matches) = matches.subcommand_matches("show") {
        show(
            matches
                .value_of("idx")
                .map(|idx| idx.parse::<usize>().unwrap()),
        )
    } else {
        app.print_help().unwrap();
        exit(1);
    }
}

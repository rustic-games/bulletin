use reqwest;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use structopt::StructOpt;
use tera::Tera;

#[derive(StructOpt, Debug)]
#[structopt(name = "bulletin")]
struct Opt {
    /// Pinboard API token
    #[structopt(short = "t", long = "token")]
    pinboard_api_token: String,

    /// Filter Pinboard posts based on coma-separated tags
    #[structopt(short = "f", long = "filter")]
    filter: Option<String>,

    /// Django template
    #[structopt(name = "TEMPLATE", parse(from_os_str))]
    template: PathBuf,
}

#[derive(Debug, Default, Serialize)]
struct Data {
    posts: Vec<Post>,
}

impl Data {
    fn from_pinboard(token: &str, filter: &str) -> Result<Self, Box<std::error::Error>> {
        let client = reqwest::Client::new();
        let mut response = client
            .get("https://api.pinboard.in/v1/posts/all")
            .query(&[("format", "json"), ("auth_token", token), ("tag", &filter)])
            .send()?;

        let body = response.text().unwrap();

        if !response.status().is_success() {
            return Err("Request to Pinboard API failed for unknown reason.".into());
        }

        let posts: Vec<Post> = serde_json::from_str(&body).unwrap();
        Ok(Data { posts })
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Post {
    href: String,
    #[serde(rename = "description")]
    title: String,
    #[serde(rename = "extended")]
    description: String,
    // tags: String,
    #[serde(deserialize_with = "from_space_delimited_string")]
    tags: HashMap<String, String>,
    time: String,
}

fn from_space_delimited_string<'de, D>(deserializer: D) -> Result<HashMap<String, String>, D::Error>
where
    D: Deserializer<'de>,
{
    let mut values = HashMap::new();
    let s: String = Deserialize::deserialize(deserializer)?;
    s.split_whitespace()
        .map(|s| s.split(':').collect())
        .filter(|v: &Vec<&str>| v.len() == 2)
        .for_each(|v| {
            values.insert(v[0].to_owned(), v[1].to_owned());
        });

    Ok(values)
}

fn main() {
    let opt = Opt::from_args();

    let template = match fs::read_to_string(opt.template) {
        Ok(string) => string,
        Err(err) => {
            eprintln!("Error reading template: {}", err);
            std::process::exit(1);
        }
    };

    let data = match Data::from_pinboard(
        &opt.pinboard_api_token,
        &opt.filter.unwrap_or_else(|| "".to_owned()),
    ) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Fetching data failed: {}", err);
            std::process::exit(1);
        }
    };

    match render(&template, data) {
        Ok(s) => println!("{}", s),
        Err(err) => {
            eprintln!("Rendering failed: {}", err);
            std::process::exit(1);
        }
    }
}

fn render(template: &str, data: Data) -> Result<String, String> {
    match Tera::one_off_value(template, &data, true) {
        Ok(s) => Ok(s),
        Err(err) => Err(format!("{}", err)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render() {
        let template = "{{ posts[0].href }}";
        let mut posts = vec![Post::default()];
        posts[0].href = "foo".to_owned();
        let data = Data { posts };

        let result = render(template, data);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "foo".to_owned());
    }
}

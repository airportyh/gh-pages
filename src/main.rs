/*

## Create new site

```
gh-pages new mysite
cd mysite
echo "<h1>Welcome to my web site!</h1>" > index.html
gh-pages publish
```
## Update the site

```
gh-pages publish
```

*/
extern crate hyper;
extern crate rustc_serialize;

use std::env;
use std::fs;
use std::process::Command;
use hyper::Client;
use std::io::Read;
use hyper::header::{Authorization, Basic, UserAgent};
use hyper::status::StatusClass;
use std::io::Write;
use rustc_serialize::json;
use rustc_serialize::json::DecoderError;
use std::fs::File;

const GH_PAGES_CLIENT_ID: & 'static str = "1de5a0305dd9acde9659";
const GH_PAGES_CLIENT_SECRET: & 'static str = "2b889f7ec694644373afd099b22601d7b81a02e0";

#[derive(RustcDecodable)]
struct GHAuthorization {
  token: String
}

#[derive(RustcEncodable, RustcDecodable)]
struct GHPagesConfig {
  username: String,
  token: String
}

#[derive(Debug)]
enum CliError {
  Io(std::io::Error),
  NoHomeDirectory,
  ConfigFileReadError(DecoderError),
  GitHubAccessError(hyper::error::Error)
}

impl From<std::io::Error> for CliError {
  fn from(err: std::io::Error) -> CliError {
    CliError::Io(err)
  }
}

impl From<DecoderError> for CliError {
  fn from(err: DecoderError) -> CliError {
    CliError::ConfigFileReadError(err)
  }
}

impl From<hyper::error::Error> for CliError {
  fn from(err: hyper::error::Error) -> CliError {
    CliError::GitHubAccessError(err)
  }
}

fn main() {
  let args: Vec<_> = env::args().collect();
  let command = &args[1];
  if command == "new" {
    let dir = &args[2];
    match read_auth().or_else(|_|authenticate()) {
      Ok(config) =>
        create_site(&dir, &config).unwrap(),
      Err(err) =>
        println!("Failed to create site - {:?}.", err)
    }
  } else if command == "publish" {
    publish_site()
      .ok()
      .expect("Failed to publish site.");
  } else {
    println!("Unknown command \"{}\".", command);
  }
}

fn read_auth() -> Result<GHPagesConfig, CliError> {
  println!("read_auth");
  let dir = try!(env::home_dir()
    .ok_or(CliError::NoHomeDirectory));
  
  let mut filepath = dir.clone();
  filepath.push(".gh-pages.json");
  let mut f = try!(File::open(&filepath));
  let mut content: String = String::new();
  try!(f.read_to_string(&mut content));
  let config: GHPagesConfig = try!(json::decode(&content));
  return Ok(config);
}

fn create_site(dir: &str, config: &GHPagesConfig) -> Result<(), CliError> {
  println!("Creating new site.");
  try!(fs::create_dir(dir));
  try!(Command::new("git")
    .arg("init")
    .current_dir(dir)
    .status());
  try!(Command::new("git")
    .current_dir(dir)
    .arg("checkout")
    .arg("-b")
    .arg("gh-pages")
    .status());
  let client = Client::new();
  let body = &format!("{{\"name\": \"{}\"}}", dir);
  let repo_url = &format!("https://github.com/{}/{}.git", 
    config.username, dir);
  let mut res = try!(client
    .post("https://api.github.com/user/repos")
    .body(body)
    .header(UserAgent("gh-pages".to_string()))
    .header(
      Authorization(
        Basic {
          username: config.username.to_owned(),
          password: Some(config.token.to_owned())
        }  
      )
    )
    .send());
  if res.status.class() != StatusClass::Success {
    let mut s = String::new();
    try!(res.read_to_string(&mut s));
    println!("Unable to create repo {} on Github.", repo_url);
    println!("{}", s);
  } else {
    println!("Succesfully created repo {}", dir);
    try!(Command::new("git")
      .current_dir(dir)
      .arg("remote")
      .arg("add")
      .arg("origin")
      .arg(repo_url)
      .status());
  } 
  println!("Done.");
  Ok(())
}

fn publish_site() -> Result<(), CliError> {
  try!(Command::new("git")
    .arg("add")
    .arg(".")
    .status());
  try!(Command::new("git")
    .arg("commit")
    .arg("-m")
    .arg("New Update.")
    .status());
  try!(Command::new("git")
    .arg("push")
    .arg("--set-upstream")
    .arg("origin")
    .arg("gh-pages")
    .status());
  Ok(())
}

fn prompt(text: &str) -> String {
  print!("{}: ", text);
  std::io::stdout().flush().unwrap();
  let mut result = String::new();
  std::io::stdin().read_line(&mut result).unwrap();
  result = result.trim_matches('\n').to_string();
  return result;
}

fn authenticate() -> Result<GHPagesConfig, CliError> {
  println!("authenticate");
  let username = prompt("Username");
  let password = prompt("Password");
  let client = Client::new();
  let body = 
    &format!(
      "{{\"scopes\": [\"repo\"], \"client_id\": \"{}\", \"client_secret\": \"{}\"}}",
      GH_PAGES_CLIENT_ID,
      GH_PAGES_CLIENT_SECRET);
  let auth = Basic {
    username: username.to_owned(),
    password: Some(password.to_owned())
  };
  let mut response = try!(client
    .post("https://api.github.com/authorizations")
    .body(body)
    .header(UserAgent("gh-pages".to_string()))
    .header(Authorization(auth))
    .send());
  let mut s = String::new();
  try!(response.read_to_string(&mut s));
  let auth: GHAuthorization = try!(json::decode(&s));
  let config = GHPagesConfig {
    username: username,
    token: auth.token
  };
  try!(save_config(&config));
  Ok(config)
}

fn save_config(config: &GHPagesConfig) -> Result<(), CliError> {
  let dir = try!(env::home_dir()
    .ok_or(CliError::NoHomeDirectory));
  let mut filepath = dir.clone();
  filepath.push(".gh-pages.json");
  let mut f = try!(File::create(&filepath));
  let json: String = json::encode(&config)
    .ok().expect("GHPages Error: cannot encode GHPagesConfig to JSON format.");
  try!(f.write_all(json.as_bytes()));
  println!("Wrote file {}", filepath.to_str().unwrap());
  Ok(())
}

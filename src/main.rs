extern crate base64;
extern crate clap;
extern crate crypto;
extern crate env_logger;
extern crate percent_encoding;
extern crate reqwest;
extern crate serde_json;
extern crate url;
#[macro_use]
extern crate log;

const VERSION_STRING: &str = "0.1.0";

use clap::{App};
mod compute;
mod keyvalue;

#[derive(Debug)]
pub struct InvalidParameter {
    name: String,
    description: String,
}

impl InvalidParameter {
    pub fn new(name: &str, description: &str) -> InvalidParameter {
        InvalidParameter {
            name: name.to_owned(),
            description: description.to_owned(),
        }
    }
    pub fn name<'a>(&'a self) -> &'a str {
        self.name.as_str()
    }
    pub fn description<'a>(&'a self) -> &'a str {
        self.description.as_str()
    }
}

#[derive(Debug)]
pub struct GenericError {
    description: String,
    location: String,
}

impl GenericError {
    pub fn new(description: &str, location: &str) -> GenericError {
        GenericError {
            description: description.to_owned(),
            location: location.to_owned()
        }
    }
    pub fn description<'a>(&'a self) -> &'a str {
        self.description.as_str()
    }
    pub fn location<'a>(&'a self) -> &'a str {
        self.location.as_str()
    }
}

#[derive(Debug)]
pub enum ApplicationError {
    IoError(std::io::Error),
    ParameterError(InvalidParameter),
    SerdeError(serde_json::Error),
    ReqwestError(reqwest::Error),
    ReqwestParseError(url::ParseError),
    GenericError(GenericError),
}

fn encode_form_url_utf8(value: &str) -> String {
    let mut ret = String::new();
    ret.reserve(value.len());
    for b in value.as_bytes() {
        if (b >= &b'a' && b <= &b'z')
            || (b >= &b'A' && b <= &b'Z')
            || (b >= &b'0' && b <= &b'9')
            || (b == &b'-')
            || (b == &b'_')
            || (b == &b'.')
        {
            let b: u8 = *b;
            ret.push(b as char);
        } else if b == &b' ' {
            ret.push('+');
        } else {
            let tmp = format!("%{:<02X}", b);
            debug!("{} = {}", b, tmp);
            ret.push_str(tmp.as_str());
        }
    }
    ret
}

fn create_app<'a, 'b>() -> App<'a, 'b> {
    App::new("IDCF client")
        .version(VERSION_STRING)
        .subcommand(compute::create_app())
}

#[tokio::main]
async fn main() -> Result<(), ApplicationError> {
    env_logger::init();
    let app = create_app().get_matches();
    match app.subcommand() {
        ("compute", Some(app)) => {
            compute::execute(app).await
        },
        _ => {
            error!("unknown subcommand");
            Err(ApplicationError::ParameterError(InvalidParameter::new("subcommand", "unknown subcommand")))
        }
    }
}

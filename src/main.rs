extern crate base64;
extern crate clap;
extern crate crypto;
extern crate percent_encoding;
extern crate reqwest;
extern crate serde_json;
extern crate url;

use clap::{App, Arg};
use crypto::mac::Mac;
use std::convert::AsRef;
use std::convert::From;
use std::iter::Iterator;

#[derive(Debug)]
struct InvalidParameter {
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
}

#[derive(Debug)]
enum ApplicationError {
    IoError(std::io::Error),
    ParameterError(InvalidParameter),
    SerdeError(serde_json::Error),
    ReqwestError(reqwest::Error),
    ReqwestParseError(reqwest::UrlError),
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
        {
            let b: u8 = *b;
            ret.push(b as char);
        } else if b == &b' ' {
            ret.push('+');
        } else {
            let tmp = format!("%{:<02X}", b);
            println!("{} = {}", b, tmp);
            ret.push_str(tmp.as_str());
        }
    }
    ret
}

fn create_querystring<TKey, TValue>(
    command: &str,
    apikey: &str,
    parameters: &Vec<(TKey, TValue)>,
) -> String
where
    TKey: std::string::ToString + std::cmp::Ord,
    TValue: std::string::ToString,
{
    let mut queryvalues: Vec<(String, String)> = parameters
        .iter()
        .map(|(x, y)| (x.to_string(), y.to_string()))
        .collect();
    queryvalues.push((String::from("command"), String::from(command)));
    queryvalues.push(("apikey".to_owned(), apikey.to_owned()));
    queryvalues.sort_by(|(x1, _), (x2, _)| x1.cmp(x2));

    let x: Vec<String> = queryvalues.iter().map(|(x, y)| {
        format!("{}={}", encode_form_url_utf8(x), encode_form_url_utf8(y))
    }).collect();
    x.join("&")
}

fn create_app<'a, 'b>() -> App<'a, 'b> {
    App::new("IDCF client")
        .version("0.1.0")
        .arg(
            Arg::with_name("apikey")
                .short("a")
                .long("apikey")
                .value_name("API_KEY")
                .help("IDCF api key"),
        )
        .arg(
            Arg::with_name("secretkey")
                .short("s")
                .long("secretkey")
                .value_name("SECRET_KEY")
                .help("IDCF secret key"),
        )
        .arg(
            Arg::with_name("input-json")
                .short("i")
                .long("input")
                .value_name("INPUT_JSON_FILE")
                .help("input keyvalue json file(cannot use with 'k' option)")
                .conflicts_with("keyvalue"),
        )
        .arg(
            Arg::with_name("keyvalue")
                .short("k")
                .long("keyvalue")
                .value_name("KEY_VALUE")
                .conflicts_with("input-json")
                .help("query keyvalue pair(A=B)(cannot use with 'i' option)")
                .multiple(true),
        )
        .arg(
            Arg::with_name("method")
                .short("m")
                .long("method")
                .value_name("METHOD")
                .required(true),
        )
        .arg(
            Arg::with_name("endpoint")
                .short("e")
                .long("endpoint")
                .required(true)
                .value_name("END_POINT"),
        )
}

fn get_keyvalue_from_json_file(filepath: &str) -> Result<Vec<(String, String)>, ApplicationError> {
    let f = match std::fs::File::open(filepath) {
        Ok(v) => v,
        Err(e) => return Err(ApplicationError::IoError(e)),
    };
    let v: serde_json::Value = match serde_json::from_reader(f) {
        Ok(v) => v,
        Err(e) => return Err(ApplicationError::SerdeError(e)),
    };
    match v {
        serde_json::Value::Object(o) => {
            let ret: Vec<(String, String)> = o
                .iter()
                .map(|(k, v)| (String::from(k), String::from(v.as_str().unwrap())))
                .collect();
            Ok(ret)
        }
        _ => Err(ApplicationError::IoError(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid json format",
        ))),
    }
}

fn get_keyvalue_from_strings(elements: &[&str]) -> Result<Vec<(String, String)>, ApplicationError> {
    let ret: Vec<(String, String)> = elements
        .iter()
        .filter(|x| x.contains("="))
        .map(|x| {
            let splitted: Vec<&str> = x.splitn(2, "=").collect();
            (splitted[0].to_owned(), splitted[1].to_owned())
        })
        .collect();
    Ok(ret)
}

fn get_signature(query_string: &str, secret_key: &str) -> Result<String, ApplicationError> {
    let hash = crypto::sha1::Sha1::new();
    let mut hmac = crypto::hmac::Hmac::new(hash, secret_key.as_bytes());
    let inputstr = query_string.to_lowercase().replace("+", "%20");
    println!("input string = {}", inputstr);
    hmac.input(inputstr.as_bytes());
    let hashed = Vec::from(hmac.result().code());
    return Ok(encode_form_url_utf8(base64::encode(&hashed).as_ref()));
}

fn get_parameters<'a>(
    app: &clap::ArgMatches<'a>,
) -> Result<Vec<(String, String)>, ApplicationError> {
    match app.value_of("input-json") {
        Some(v) => Ok(get_keyvalue_from_json_file(v)?),
        None => match app.values_of("keyvalue") {
            Some(v) => {
                let vec: Vec<&str> = v.collect();
                get_keyvalue_from_strings(&vec)
            }
            None => Ok(Vec::new() as Vec<(String, String)>),
        },
    }
}

fn main() -> Result<(), ApplicationError> {
    let app = create_app().get_matches();
    let mut parameters = get_parameters(&app)?;
    let method = match app.value_of("method") {
        Some(v) => v.to_owned(),
        None => {
            return Err(ApplicationError::ParameterError(InvalidParameter::new(
                "method",
                "you must set method option",
            )))
        }
    };
    let apikey = match app.value_of("apikey") {
        Some(v) => v.to_owned(),
        None => match std::env::var("IDCF_API_KEY") {
            Ok(v) => v,
            Err(_) => {
                return Err(ApplicationError::ParameterError(InvalidParameter::new(
                    "apikey",
                    "you must set API key option or IDCF_API_KEY env variable",
                )))
            }
        },
    };
    let secretkey = match app.value_of("secretkey") {
        Some(v) => v.to_owned(),
        None => match std::env::var("IDCF_SECRET_KEY") {
            Ok(v) => v,
            Err(_) => {
                return Err(ApplicationError::ParameterError(InvalidParameter::new(
                    "apikey",
                    "you must set API key option or IDCF_SECRET_KEY env variable",
                )))
            }
        },
    };
    let endpoint = match app.value_of("endpoint") {
        Some(v) => v.to_owned(),
        None => {
            return Err(ApplicationError::ParameterError(InvalidParameter::new(
                "endpoint",
                "you must set endpoint",
            )))
        }
    };
    parameters.sort_by(|(x1, _), (x2, _)| x1.cmp(x2));
    let query_string = create_querystring(method.as_str(), apikey.as_str(), &parameters);
    let signature = get_signature(&query_string, &secretkey)?;
    let client: reqwest::Client = reqwest::ClientBuilder::new()
        .use_default_tls()
        .use_sys_proxy()
        .redirect(reqwest::RedirectPolicy::default())
        .build()
        .unwrap();
    let requesturl = match reqwest::Url::parse(
        format!("{}?{}&signature={}", endpoint, query_string, signature).as_ref(),
    ) {
        Ok(v) => v,
        Err(e) => return Err(ApplicationError::ReqwestParseError(e)),
    };
    match client.request(reqwest::Method::POST, requesturl).send() {
        Ok(v) => {
            println!("request success:{:?}", v);
        }
        Err(e) => {
            println!("reqwest error:{:?}", e);
            return Err(ApplicationError::ReqwestError(e));
        }
    };
    Ok(())
}

extern crate base64;
extern crate clap;
extern crate crypto;
extern crate env_logger;
extern crate log;
extern crate percent_encoding;
extern crate reqwest;
extern crate serde_json;
extern crate url;

use clap::{App, Arg};
use crypto::mac::Mac;
use std::convert::AsRef;
use std::convert::From;
use std::io::Read;
use std::iter::Iterator;

use super::encode_form_url_utf8;
use super::ApplicationError;
use super::InvalidParameter;
use super::GenericError;

use super::keyvalue;

struct CommandOptions {
    method: String,
    apikey: String,
    secretkey: String,
    endpoint: String,
    output_path: Option<String>,
}

impl CommandOptions {
    pub fn output_path(&self) -> Option<&str> {
        match &self.output_path {
            Some(v) => Some(v.as_str()),
            None => None,
        }
    }
}

fn get_command_option(app: &clap::ArgMatches) -> Result<CommandOptions, ApplicationError> {
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
        None => match std::env::var("IDCF_ENDPOINT") {
            Ok(v) => v,
            Err(e) => {
                return Err(ApplicationError::ParameterError(InvalidParameter::new(
                    "endpoint",
                    format!("you must set endpoint({:?})", e).as_str(),
                )))
            }
        },
    };
    let output_path = match app.value_of("output") {
        Some(v) => Some(v.to_owned()),
        None => None,
    };
    Ok(CommandOptions {
        method: method,
        apikey: apikey,
        secretkey: secretkey,
        endpoint: endpoint,
        output_path: output_path,
    })
}

pub fn create_app<'a, 'b>() -> App<'a, 'b> {
    App::new("compute")
        .about("IDCF compute API client")
        .after_help("you can get detailed API reference in https://www.idcf.jp/api-docs/apis/?id=docs_compute_reference")
        .version(super::VERSION_STRING)
        .arg(
            Arg::with_name("apikey")
                .short("a")
                .long("apikey")
                .value_name("API_KEY")
                .help("IDCF api key, if not set, using IDCF_API_KEY environment variable"),
        )
        .arg(
            Arg::with_name("secretkey")
                .short("s")
                .long("secretkey")
                .value_name("SECRET_KEY")
                .help("IDCF secret key, if not set, using IDCF_SECRET_KEY environment variable"),
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
                .help("API method name, REQUIRED")
                .required(true),
        )
        .arg(
            Arg::with_name("endpoint")
                .short("e")
                .long("endpoint")
                .value_name("END_POINT")
                .help("if not set, IDCF_ENDPOINT environment variable will be used"),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .value_name("OUTPUT_PATH")
                .help("output file path, if not set, output to stdout"),
        )
}

fn get_signature(query_string: &str, secret_key: &str) -> Result<String, ApplicationError> {
    let hash = crypto::sha1::Sha1::new();
    let mut hmac = crypto::hmac::Hmac::new(hash, secret_key.as_bytes());
    let inputstr = query_string.to_lowercase()
        .replace("+", "%20")
        .replace("%2a", "*")
        .replace("%5b", "[")
        .replace("%5d", "]")
        .replace("%2e", ".");
    info!("signature input string = {}, {}", query_string, inputstr);
    hmac.input(inputstr.as_bytes());
    let hashed = Vec::from(hmac.result().code());
    return Ok(encode_form_url_utf8(base64::encode(&hashed).as_ref()));
}

fn get_parameters<'a>(
    app: &clap::ArgMatches<'a>,
) -> Result<Vec<(String, String)>, ApplicationError> {
    match app.value_of("input-json") {
        Some(v) => Ok(keyvalue::get_keyvalue_from_json_file(v)?),
        None => match app.values_of("keyvalue") {
            Some(v) => {
                let vec: Vec<&str> = v.collect();
                keyvalue::get_keyvalue_from_strings(&vec)
            }
            None => Ok(Vec::new() as Vec<(String, String)>),
        },
    }
}

fn output_response_to_stream<T>(w: &mut T, mut res: reqwest::Response) -> Result<(), std::io::Error>
where
    T: std::io::Write,
{
    let mut buf = [0u8; 4096];
    loop {
        let bytesread = res.read(&mut buf)?;
        if bytesread <= 0 {
            break;
        }
        w.write(&buf[0..bytesread])?;
    }
    Ok(())
}

fn output_response(
    res: reqwest::Response,
    output_path: Option<&str>,
) -> Result<(), ApplicationError> {
    if let Some(file_path) = output_path {
        let mut f = match std::fs::File::create(file_path) {
            Ok(v) => v,
            Err(e) => return Err(ApplicationError::IoError(e)),
        };
        match output_response_to_stream(&mut f, res) {
            Ok(_) => (),
            Err(e) => return Err(ApplicationError::IoError(e)),
        };
    } else {
        let mut stdout = std::io::stdout();
        match output_response_to_stream(&mut stdout, res) {
            Ok(_) => (),
            Err(e) => return Err(ApplicationError::IoError(e)),
        };
    }
    Ok(())
}

pub fn execute<'a>(app: &clap::ArgMatches<'a>) -> Result<(), ApplicationError> {
    let mut parameters = get_parameters(&app)?;
    let option = get_command_option(&app)?;
    parameters.sort_by(|(x1, _), (x2, _)| x1.cmp(x2));
    let query_string =
        keyvalue::create_querystring(option.method.as_str(), option.apikey.as_str(), &parameters);
    info!("querystring = {}", query_string);
    let signature = get_signature(&query_string, &option.secretkey)?;
    let client: reqwest::Client = reqwest::ClientBuilder::new()
        .use_default_tls()
        .use_sys_proxy()
        .redirect(reqwest::RedirectPolicy::default())
        .build()
        .unwrap();
    let requesturl = match reqwest::Url::parse(
        format!(
            "{}?{}&signature={}",
            option.endpoint, query_string, signature
        )
        .as_ref(),
    ) {
        Ok(v) => v,
        Err(e) => return Err(ApplicationError::ReqwestParseError(e)),
    };
    match client
        .request(reqwest::Method::GET, requesturl)
        .send()
    {
        Ok(v) => {
            let mut v: reqwest::Response = v;
            if v.status().is_success() {
                info!("request success:{:?}", v);
                output_response(v, option.output_path())?;
            } else {
                eprintln!("response error:{:?}", v);
                // output_response(v, option.output_path())?;
                let mut body = String::new();
                v.read_to_string(&mut body).unwrap();
                return Err(ApplicationError::GenericError(GenericError::new(format!("response error:{}, {}", v.status(), body).as_str(), "compute::execute")))
            }
        }
        Err(e) => {
            eprintln!("reqwest error:{:?}", e);
            return Err(ApplicationError::ReqwestError(e));
        }
    };
    Ok(())
}
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
use std::iter::Iterator;

use super::encode_form_url_utf8;
use super::ApplicationError;
use super::GenericError;
use super::InvalidParameter;

use super::keyvalue;

struct CommandOptions {
    method: String,
    apikey: String,
    secretkey: String,
    endpoint: String,
    output_path: Option<String>,
    is_json: bool,
}

impl CommandOptions {
    pub fn output_path(&self) -> Option<&str> {
        match &self.output_path {
            Some(v) => Some(v.as_str()),
            None => None,
        }
    }
}

fn get_value_from_cmd_and_env(
    app: &clap::ArgMatches,
    name: &str,
    first: &str,
    second: &str,
    errmsg: &str,
) -> Result<String, ApplicationError> {
    match app.value_of(name) {
        Some(v) => Ok(v.to_owned()),
        None => match std::env::var(first) {
            Ok(v) => Ok(v.to_owned()),
            Err(_) => match std::env::var(second) {
                Ok(v) => Ok(v.to_owned()),
                Err(e) => Err(ApplicationError::ParameterError(InvalidParameter::new(
                    name,
                    (format!("{}: ({:?})", errmsg, e)).as_str(),
                ))),
            },
        },
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
    let apikey = get_value_from_cmd_and_env(
        app,
        "apikey",
        "IDCF_API_KEY",
        "CLOUDSTACK_API_KEY",
        "you must set API key option or IDCF_API_KEY or CLOUDSTACK_API_KEY env variable",
    )?;
    let secretkey = get_value_from_cmd_and_env(
        app,
        "secretkey",
        "IDCF_SECRET_KEY",
        "CLOUDSTACK_SECRET_KEY",
        "you must set API key option or IDCF_SECRET_KEY or CLOUDSTACK_SECRET_KEY env variable",
    )?;
    let endpoint = get_value_from_cmd_and_env(app, "endpoint", "IDCF_ENDPOINT", "CLOUDSTACK_ENDPOINT", "you must set endpoint by parameter(--endpoint) or IDCF_ENDPOINT or CLOUDSTACK_ENDPOINT env variable")?;
    let output_path = match app.value_of("output") {
        Some(v) => Some(v.to_owned()),
        None => None,
    };
    let format = match app.value_of("output-format") {
        Some(v) => match v {
            "xml" => "xml".to_owned(),
            "json" => "json".to_owned(),
            _ => {
                return Err(ApplicationError::ParameterError(InvalidParameter::new(
                    "output-format",
                    "unknown output format",
                )))
            }
        },
        None => "xml".to_owned(),
    };
    Ok(CommandOptions {
        method: method,
        apikey: apikey,
        secretkey: secretkey,
        endpoint: endpoint,
        output_path: output_path,
        is_json: format == "json",
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
        .arg(
            Arg::with_name("output-format")
                .short("f")
                .long("output-format")
                .value_name("OUTPUT_FORMAT")
                .help("output format(currently, xml(default) and json was supported"),
        )
}

fn get_signature(query_string: &str, secret_key: &str) -> Result<String, ApplicationError> {
    let hash = crypto::sha1::Sha1::new();
    let mut hmac = crypto::hmac::Hmac::new(hash, secret_key.as_bytes());
    let inputstr = query_string
        .to_lowercase()
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

async fn output_response_to_stream<T>(w: &mut T, res: reqwest::Response) -> Result<(), ApplicationError>
where
    T: std::io::Write,
{
    let bytes = match res.bytes().await {
        Ok(v) => v,
        Err(e) => return Err(ApplicationError::ReqwestError(e))
    };
    match w.write(&bytes) {
        Ok(_) => (),
        Err(e) => return Err(ApplicationError::IoError(e)),
    };
    Ok(())
}

async fn output_response(
    res: reqwest::Response,
    output_path: Option<&str>,
) -> Result<(), ApplicationError> {
    if let Some(file_path) = output_path {
        let mut f = match std::fs::File::create(file_path) {
            Ok(v) => v,
            Err(e) => return Err(ApplicationError::IoError(e)),
        };
        output_response_to_stream(&mut f, res).await?;
    } else {
        let mut stdout = std::io::stdout();
        output_response_to_stream(&mut stdout, res).await?;
    }
    Ok(())
}

pub async fn execute<'a>(app: &clap::ArgMatches<'a>) -> Result<(), ApplicationError> {
    let mut parameters = get_parameters(&app)?;
    let option = get_command_option(&app)?;
    parameters.sort_by(|(x1, _), (x2, _)| x1.cmp(x2));
    let query_string = keyvalue::create_querystring(
        option.method.as_str(),
        option.apikey.as_str(),
        option.is_json,
        &parameters,
    );
    info!("querystring = {}", query_string);
    let signature = get_signature(&query_string, &option.secretkey)?;
    let client: reqwest::Client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::default())
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
    match client.request(reqwest::Method::GET, requesturl).send().await {
        Ok(v) => {
            let v: reqwest::Response = v;
            let status = v.status();

            if status.is_success() {
                info!("request success:{:?}", v);
                output_response(v, option.output_path()).await?;
            } else {
                eprintln!("response error:{:?}", v);
                // output_response(v, option.output_path())?;
                let body = match v.text().await {
                    Ok(v) => v,
                    Err(e) => return Err(ApplicationError::ReqwestError(e)),
                };
                return Err(ApplicationError::GenericError(GenericError::new(
                    format!("response error:{}, {}", status, body).as_str(),
                    "compute::execute",
                )));
            }
        }
        Err(e) => {
            eprintln!("reqwest error:{:?}", e);
            return Err(ApplicationError::ReqwestError(e));
        }
    };
    Ok(())
}

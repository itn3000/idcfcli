extern crate reqwest;
extern crate clap;
extern crate crypto;
extern crate percent_encoding;

use std::collections::HashMap;
use std::string::ToString;
use percent_encoding::AsciiSet;

fn create_querystring<T>(command: &str, apikey: &str, parameters: Vec<(T, T)>) -> String where T: std::string::ToString+std::cmp::Ord {
    let mut ret = String::from(format!("command={}", command));
    ret.insert_str(ret.len(), apikey);
    const MY_ASCIISET: AsciiSet = percent_encoding::CONTROLS
        .add(b' ')
        .add(b'!')
        .add(b'=')
        .add(b'&')
        .add(b'+')
        ;
    let sorted = parameters.sort_by(|(x1, _), (x2, _)| x1.cmp(x2));
    for (key, value) in parameters {
        let key = key.to_string();
        let value = value.to_string();
        let x = percent_encoding::utf8_percent_encode(key.as_str(), &MY_ASCIISET);
        let y = percent_encoding::utf8_percent_encode(value.as_str(), &MY_ASCIISET);
        ret.push('&');
        ret.push_str(format!("{}={}", x, y).as_ref());
    }
    ret
}

fn main() {
    let client: reqwest::Client = reqwest::ClientBuilder::new()
        .use_default_tls()
        .use_sys_proxy()
        .redirect(reqwest::RedirectPolicy::default())
        .build().unwrap();
    client.request(reqwest::Method::POST, "https://www.google.co.jp")
        .query(&[("", "")]);
}

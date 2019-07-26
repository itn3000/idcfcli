extern crate serde_json;

use super::{ApplicationError, GenericError};

pub fn get_keyvalue_from_strings(elements: &[&str]) -> Result<Vec<(String, String)>, ApplicationError> {
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

pub fn get_keyvalue_from_json_file(filepath: &str) -> Result<Vec<(String, String)>, ApplicationError> {
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
                .map(|(k, v)| (String::from(k), match v {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Array(ar) => serde_json::to_string(ar).unwrap(),
                    serde_json::Value::Number(num) => serde_json::to_string(num).unwrap(),
                    serde_json::Value::Object(obj) => serde_json::to_string(obj).unwrap(),
                    serde_json::Value::Bool(b) => serde_json::to_string(b).unwrap(),
                    serde_json::Value::Null => String::from("null"),
                }
                ))
                .collect();
            Ok(ret)
        }
        _ => Err(ApplicationError::IoError(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid json format",
        ))),
    }
}

pub fn create_querystring<TKey, TValue>(
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
    queryvalues.push(("response".to_owned(), "json".to_owned()));
    queryvalues.sort_by(|(x1, _), (x2, _)| x1.cmp(x2));

    let x: Vec<String> = queryvalues
        .iter()
        .map(|(x, y)| {
            format!(
                "{}={}",
                super::encode_form_url_utf8(x),
                super::encode_form_url_utf8(y)
            )
        })
        .collect();
    x.join("&")
}


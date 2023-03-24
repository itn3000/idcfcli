# Overview

This is [IDCF](https://www.idcf.jp) API client written in Rust.
Currently, only computing API is supported.
Entire API reference is in [IDCF Cloud API Docs](https://www.idcf.jp/api-docs/)

# Usage

This is the single executable binary, so you download it from [release page](https://github.com/itn3000/idcfcli/releases), then you can execute command.
you can get global help by `--help` option. Here is the output.

```
IDCF client 0.3.0

USAGE:
    idcfcli.exe [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    compute    IDCF compute API client
    help       Prints this message or the help of the given subcommand(s)
```

# Usage of compute

IDCF compute API is related on manupilating IDCF computing(Virtual Machines).
you can get help by `compute --help`.Here is the output.

```
idcfcli.exe-compute 0.3.0
IDCF compute API client

USAGE:
    idcfcli.exe compute [OPTIONS] --method <METHOD>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -a, --apikey <API_KEY>                 IDCF api key, if not set, using IDCF_API_KEY environment variable
    -e, --endpoint <END_POINT>             if not set, IDCF_ENDPOINT environment variable will be used
    -i, --input <INPUT_JSON_FILE>          input keyvalue json file(cannot use with 'k' option)
    -k, --keyvalue <KEY_VALUE>...          query keyvalue pair(A=B)(cannot use with 'i' option)
    -m, --method <METHOD>                  API method name, REQUIRED
    -o, --output <OUTPUT_PATH>             output file path, if not set, output to stdout
    -f, --output-format <OUTPUT_FORMAT>    output format(currently, xml(default) and json was supported
    -s, --secretkey <SECRET_KEY>           IDCF secret key, if not set, using IDCF_SECRET_KEY environment variable

you can get detailed API reference in https://www.idcf.jp/api-docs/apis/?id=docs_compute_reference
```

## Warning

Some API is asynchronous, you must call queryAsyncJobResult after API request.

## Examples

### Get list of templates

`idcfcli compute -m listTemplates -k templatefilter=featured -s [SECRET_KEY] -a [API_KEY]`

### Set display name to public IP address resource by JSON input file

1. creating following JSON file.
```json
{
    "resourceids":"[resource ID]",
    "resourceType":"PublicIpAddress",
    "tags[0].key":"cloud-description",
    "tags[0].value":"[desired name]"
}
```
2. execute `idcfcli compute -m createTags -i path/to/json -s [SECRET_KEY] -a [API_KEY]`
3. save output job ID
    `{"createtagsresponse":{"jobid":"[job ID]"}}`
4. execute `idcfcli compute -m queryAsyncJobResult -k jobid=[job ID] -s [SECRET_KEY] -a [API_KEY]`
    Asynchronous API must be called [queryAsyncJobResult](https://cloudstack.apache.org/api/apidocs-4.8/user/queryAsyncJobResult.html) to complete job.


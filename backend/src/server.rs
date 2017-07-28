use std::error::Error;
use std::io::Cursor;

use rocket::{self, State, Request, Response};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::{Header, ContentType, Method};
use rocket_contrib::Json;
use rocket::response::Content;
use serde_json::{self};

use ::db::{Db, BaseData, Command, TranslationData};



pub struct CORS;

impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to requests",
            kind: Kind::Response
        }
    }

    fn on_response(&self, request: &Request, response: &mut Response) {
        if request.method() == Method::Options || response.content_type() == Some(ContentType::JSON) {
            response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
            response.set_header(Header::new("Access-Control-Allow-Methods", "POST, GET, OPTIONS"));
            response.set_header(Header::new("Access-Control-Allow-Headers", "Content-Type"));
            response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
        }

        if request.method() == Method::Options {
            response.set_header(ContentType::Plain);
            response.set_sized_body(Cursor::new(""));
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommandError {
    code: String,
}

#[derive(Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum CommandResult {
    Ok {
        data: ::serde_json::Value,
    },
    Error {
        error: CommandError,
    },
}

#[get("/")]
fn index() -> Content<&'static str> {
    let index_file = include_bytes!("../../frontend/dist/index.html");
    let content = ::std::str::from_utf8(&index_file[0..]).unwrap();

    Content(ContentType::HTML, content)
}

#[get("/assets/bundle.js")]
fn assets_js() -> Content<&'static str> {
    let js_bundle = include_bytes!("../../frontend/dist/assets/bundle.js");
    let content = ::std::str::from_utf8(&js_bundle[0..]).unwrap();

    Content(ContentType::JavaScript, content)
}

#[derive(FromForm)]
struct ExportArgs {
    format: Option<String>,
    pretty: Option<bool>,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
enum ExportFormat {
    Json,
    Javascript,
}

impl ExportFormat {
    fn from_str(s: &str) -> Option<Self> {
        match s.trim() {
            "json" => Some(ExportFormat::Json),
            "javascript" => Some(ExportFormat::Javascript),
            _ => None,
        }
    }
}

#[get("/export/translations/<lang>?<args>")]
fn export_translations(lang: String, args: ExportArgs, db: State<Db>)
    -> Result<Content<String>, Box<Error>>
{
    let format = args.format.and_then(|x| ExportFormat::from_str(&x)).unwrap_or(ExportFormat::Json);
    let pretty = args.pretty.unwrap_or(false);

    let data = db.translations_export(lang)?;
    let mut json = if pretty {
        serde_json::to_string_pretty(&data)?
    } else {
        serde_json::to_string(&data)?
    };

    if format == ExportFormat::Javascript {
        json = format!(
            "// This file was auto-generated. Do not edit by hand!\n\n/* tslint:disable */\n\nexport const translations = {};\n\nexport default translations;\n",
            json);
    }

    Ok(Content(ContentType::JSON, json))
}

#[get("/export/keys?<args>")]
fn export_keys(args: ExportArgs, db: State<Db>) -> Result<Content<String>, Box<Error>> {
    let format = args.format.and_then(|x| ExportFormat::from_str(&x)).unwrap_or(ExportFormat::Json);
    let pretty = args.pretty.unwrap_or(false);

    let tree = db.build_key_tree()?;
    let data = tree.to_json_value();
    let mut json = if pretty {
        serde_json::to_string_pretty(&data)?
    } else {
        serde_json::to_string(&data)?
    };

    if format == ExportFormat::Javascript {
        json = format!(
            "// This file was auto-generated. Do not edit by hand!\n\n/* tslint:disable */\n\nexport const intlKeys = {};\n\nexport default intlKeys;\n",
            json);
    }

    Ok(Content(ContentType::JSON, json))
}

#[get("/api/base-data")]
fn api_base_data(db: State<Db>) -> Result<Json<BaseData>, Box<Error>> {
  db.base_data()
    .map(|d| Json(d))
}

#[get("/api/translations/<key>")]
fn api_translations(key: String, db: State<Db>)
  -> Result<Json<TranslationData>, Box<Error>>
{
  db.translations(key)
    .map(|d| Json(d))
}

#[post("/api/command", data="<cmd>")]
fn api_command(cmd: Json<Command>, db: State<Db>)
  -> Json<CommandResult>
{
    let res = match db.command(&*cmd) {
        Ok(data) => CommandResult::Ok {
            data: json!(data),
        },
        Err(e) => CommandResult::Error{
            error: CommandError{
                code: e.to_string(),
            },
        },
    };
    Json(res)
}

#[options("/api/command")]
fn api_command_options() -> &'static str {
    ""
}

pub fn build_rocket() -> rocket::Rocket {
    let db = Db::new();
    rocket::ignite()
        .attach(CORS)
        .manage(db)
        .mount("/", routes![
            index,
            export_translations,
            export_keys,
            api_base_data,
            api_translations,
            api_command,
            api_command_options,
            assets_js,
        ])
}

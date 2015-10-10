extern crate rustc_serialize;
extern crate hyper;

use rustc_serialize::json::{ToJson, Json};
use self::hyper::Client as HyperClient;
use self::hyper::header::{Authorization, Bearer, UserAgent, ContentType};
use self::hyper::status::StatusCode;

use std::collections::BTreeMap;
use std::env;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use error::Error;

const GIST_API:          &'static str = "https://api.github.com/gists";
const GITHUB_TOKEN:      &'static str = "GITHUB_TOKEN";
const USER_AGENT:        &'static str = "Pepito Gist";

pub struct GistFile {
    name:     String,
    contents: String,
}

pub struct Gist {
    anonymous: bool,
    public:    bool,
    files:     Vec<GistFile>,
    token:     String,
}

impl Gist {
    pub fn new(public: bool, anonymous: bool) -> Gist {
        let token = env::var(&GITHUB_TOKEN.to_string());
        if token.is_err() && !anonymous {
            panic!("Missing GITHUB_TOKEN environment variable.");
        }

        Gist {
            token:     token.unwrap_or("".to_string()),
            anonymous: anonymous,
            public:    public,
            files:     vec![],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    // Add a file.
    pub fn add_file(&mut self, gist : GistFile) {
        self.files.push(gist);
    }

    // Sent to Github.
    pub fn create(&mut self) -> Result<String, Error> {
        let client    = HyperClient::new();
        let json_body = self.to_json().to_string();
        let uri       = &GIST_API.to_string();

        let mut req = client.post(uri);
        if !self.anonymous {
            req = req.header(Authorization(Bearer { token: self.token.to_owned() }))
        }

        let mut res = try!(req.header(UserAgent(USER_AGENT.to_owned()))
                              .header(ContentType::json())
                              .body(json_body.as_bytes())
                              .send());
        if res.status == StatusCode::Created {
            let mut body = String::new();
            try!(res.read_to_string(&mut body));
            return Ok(body);
        }
        Err(Error::ApiError)
    }
}

impl GistFile {
    pub fn new(name: String) -> GistFile {
        GistFile {
            name:     name,
            contents: String::new(),
        }
    }

    // Read standard input to contents buffer.
    pub fn read_stdin(&mut self) -> Result<(), Error> {
        try!(io::stdin().read_to_string(&mut self.contents));
        Ok(())
    }

    // Read file to contents buffer.
    pub fn read_file(&mut self) -> Result<(), Error> {
        let path = Path::new(&self.name);
        let mut fh = try!(File::open(&path));
        try!(fh.read_to_string(&mut self.contents));
        Ok(())
    }
}

impl ToJson for GistFile {
    fn to_json(&self) -> Json {
        let mut root = BTreeMap::new();
        root.insert("content".to_string(), self.contents.to_json());
        Json::Object(root)
    }
}

impl ToJson for Gist {
    fn to_json(&self) -> Json {
        let mut root  = BTreeMap::new();
        let mut files = BTreeMap::new();

        root.insert("public".to_string(), self.public.to_json());
        for g in self.files.iter() {
            // Remove directories from file path on serialization.
            let v: Vec<&str> = g.name.split('/').collect();
            let name: String = v.last().unwrap().to_string();
            files.insert(name, g.to_json());
        }
        root.insert("files".to_string(), files.to_json());
        Json::Object(root)
    }
}

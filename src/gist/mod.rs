pub mod gist_file;

extern crate rustc_serialize;
extern crate hyper;

use self::rustc_serialize::json::{ToJson, Json};
use self::hyper::Client as HyperClient;
use self::hyper::header::{Authorization, Bearer, UserAgent, ContentType};
use self::hyper::status::StatusCode;

use std::collections::BTreeMap;
use std::env;
use std::io::Read;

const GIST_API: &'static str = "https://api.github.com/gists";
const GITHUB_TOKEN: &'static str = "GITHUB_TOKEN";
const USER_AGENT: &'static str = "Pepito Gist";

pub struct Gist {
    anonymous: bool,
    public: bool,
    files: Vec<gist_file::GistFile>,
    token: String,
}

impl Gist {
    pub fn new(public: bool, anonymous: bool) -> Gist {
        let token = env::var(GITHUB_TOKEN);
        if token.is_err() && !anonymous {
            panic!("Missing GITHUB_TOKEN environment variable.");
        }

        Gist {
            token: token.unwrap_or("".to_string()),
            anonymous: anonymous,
            public: public,
            files: vec![],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    // Add a file.
    pub fn add_file(&mut self, gist: gist_file::GistFile) {
        self.files.push(gist);
    }

    // Sent to Github.
    pub fn create(&mut self) -> Result<String, String> {
        let client = HyperClient::new();
        let json_body = self.to_json().to_string();
        let uri = &GIST_API.to_string();

        let mut req = client.post(uri);
        if !self.anonymous {
            req = req.header(Authorization(Bearer { token: self.token.to_owned() }))
        }

        let mut res = req.header(UserAgent(USER_AGENT.to_owned()))
                         .header(ContentType::json())
                         .body(json_body.as_bytes())
                         .send()
                         .unwrap();
        if res.status == StatusCode::Created {
            let mut body = String::new();
            res.read_to_string(&mut body).unwrap();
            return Ok(body);
        }
        Err("API error".to_owned())
    }
}

impl ToJson for Gist {
    fn to_json(&self) -> Json {
        let mut root = BTreeMap::new();
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

#[cfg(test)]
mod tests {
    use super::*;
    use super::rustc_serialize::json::ToJson;

    fn fake_gist_file(contents: Option<&str>) -> gist_file::GistFile {
        let mut f = gist_file::GistFile::new("/path/to/file.txt".to_string());
        if contents.is_some() {
            f.contents = contents.unwrap().to_string();
        }
        return f;
    }

    #[test]
    fn add_files() {
        let mut g = Gist::new(true, true);
        g.add_file(fake_gist_file(None));
        g.add_file(fake_gist_file(None));
        assert_eq!(g.files.len(), 2);
    }

    #[test]
    fn emptyness() {
        let mut g = Gist::new(true, true);
        assert!(g.is_empty());

        g.add_file(fake_gist_file(None));
        assert!(!g.is_empty());
    }

    #[test]
    fn public_json() {
        let mut public = Gist::new(true, true);
        public.add_file(fake_gist_file(Some("public file contents")));

        let public_json = public.to_json().to_string();
        assert_eq!(public_json,
                   "{\"files\":{\"file.txt\":{\"content\":\"public file \
                    contents\"}},\"public\":true}");
    }

    #[test]
    fn private_json() {
        let mut private = Gist::new(false, true);
        private.add_file(fake_gist_file(Some("private file contents")));

        let private_json = private.to_json().to_string();
        assert_eq!(private_json,
                   "{\"files\":{\"file.txt\":{\"content\":\"private file \
                    contents\"}},\"public\":false}");
    }
}

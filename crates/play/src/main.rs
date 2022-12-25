#![feature(proc_macro_hygiene, decl_macro)]
#![feature(default_free_fn)]

#[macro_use]
extern crate rocket;

use rocket::Config;
use rocket::fs::FileServer;
use std::path;

#[launch]
fn web() -> _ {
    let port = 80;

    let mut config = Config::default();

    config.address = "0.0.0.0".parse().unwrap();
    config.port = port;

    let mut root_directory = path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root_directory.pop(); //move to crates
    root_directory.pop(); //move to boson
    
    let mut engine_directory = root_directory.clone();
    engine_directory.push("crates");
    engine_directory.push("engine");
    engine_directory.push("pkg");

    let mut static_directory = root_directory.clone();
    static_directory.push("crates");
    static_directory.push("play");
    static_directory.push("static");

    rocket::custom(config)
        .mount("/play", FileServer::from(static_directory.to_str().unwrap()))
        .mount("/engine", FileServer::from(engine_directory.to_str().unwrap()))
}

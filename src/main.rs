extern crate docopt;
extern crate sendgrid;
extern crate serde;
extern crate sqlite;
extern crate xml;

use sendgrid::Mail;
use sendgrid::SGClient;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
use xml::reader::EventReader;
use xml::reader::XmlEvent;

const USAGE: &'static str = "
Usage:
    rss2email add <url>
    rss2email list
    rss2email remove <pk>
    rss2email poll
";

#[derive(serde::Deserialize)]
struct Args {
    arg_pk: i32,
    arg_url: String,
    cmd_add: bool,
    cmd_list: bool,
    cmd_poll: bool,
    cmd_remove: bool,
}

#[derive(Hash)]
struct Item {
    desc: String,
    link: String,
    title: String
}

impl Item {
    fn new() -> Item {
        Item {
            desc: "".to_string(),
            link: "".to_string(),
            title: "".to_string()
        }
    }
}

fn connect() -> sqlite::Connection {
    let conn = sqlite::open("db/db.sqlite").unwrap();

    conn.execute("
        CREATE TABLE IF NOT EXISTS feeds (
            pk INTEGER PRIMARY KEY,
            url TEXT NOT NULL
        )
    ").unwrap();

    conn.execute("
        CREATE TABLE IF NOT EXISTS items (
            pk INTEGER PRIMARY KEY,
            hash TEXT NOT NULL
        )
    ").unwrap();

    return conn;
}

fn add(url: String) {
    let sql = format!("
        INSERT INTO feeds (url)
        VALUES ('{}')
    ", url);

    let conn = connect();
    conn.execute(sql).unwrap();
}

fn list() {
    let sql = format!("
        SELECT pk, url
        FROM feeds
    ");

    let conn = connect();
    conn.iterate(sql, |pairs| {
        for &(_, value) in pairs.iter() {
            print!("{} ", value.unwrap());
        }
        println!();
        true
    }).unwrap();
}

fn poll(client: &SGClient, url: String) {
    let resp = reqwest::blocking::get(&url).unwrap();
    let body = resp.text().unwrap();
    let parser = EventReader::new(body.as_bytes());
    let mut item = Item::new();
    let mut el = String::new();
    for event in parser {
        match event {
            Ok(XmlEvent::StartElement { name, .. }) => {
                el = name.local_name;
            },
            Ok(XmlEvent::CData(text)) | Ok(XmlEvent::Characters(text)) => {
                if el == "title" {
                    item.title = text;
                } else if el == "description" {
                    item.desc = text;
                } else if el == "link" {
                    item.link = text;
                }
            },
            Ok(XmlEvent::EndElement { name }) => {
                if name.local_name == "item" {
                    let mut hasher = DefaultHasher::new();
                    item.hash(&mut hasher);
                    let hash = hasher.finish();

                    let conn = connect();
                    let mut cursor = conn.prepare(format!("
                        SELECT 1 FROM items
                        WHERE hash = '{}'
                    ", hash)).unwrap().cursor();
                    match cursor.next() {
                        Ok(result) => {
                            match result {
                                Some(_) => {
                                    println!("skipping {}", hash)
                                },
                                None => {
                                    println!("sending {}", hash);
                                    let sql = format!("
                                        INSERT INTO items (hash)
                                        VALUES ('{}')
                                    ", hash);
                                    conn.execute(sql).unwrap();

                                    let subject = format!("
                                        [rss2email] {}
                                    ", item.title);
                                    let text = format!("
                                        <div>
                                            <h1>
                                                <a href='{}'>{}</a>
                                            </h1>
                                            <p>
                                                {}
                                            </p>
                                        </div>
                                    ", item.link, item.title, item.desc);
                                    let mail = Mail::new()
                                        .add_from("jan@kroeze.io")
                                        .add_html(&text)
                                        .add_subject(&subject)
                                        .add_to(("jcwkroeze@pm.me", "Jan CW Kroeze").into());
                                    client.send(mail).unwrap();
                                }
                            }
                        },
                        Err(e) => {
                            println!("{}", e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("Error: {}", e);
                return;
            },
            _ => {}
        }
    }
}

fn poll_all() {
    let my_secret_key = std::env::var("SENDGRID_KEY").expect("need SENDGRID_KEY to test");
    let client = SGClient::new(my_secret_key);

    let sql = format!("
        SELECT url
        FROM feeds
    ");

    let mut urls = Vec::new();
    let conn = connect();
    conn.iterate(sql, |pairs| {
        for &(_, value) in pairs.iter() {
            urls.push(value.unwrap().to_string());
        }
        true
    }).unwrap();

    for url in urls {
        poll(&client, url);
    }
}

fn remove(pk: i32) {
    let sql = format!("
        DELETE FROM feeds
        WHERE pk = {}
    ", pk);

    let conn = connect();
    conn.execute(sql).unwrap();
}

fn main() {
    let args: Args = docopt::Docopt::new(USAGE)
        .and_then(|d| d.argv(std::env::args().into_iter()).deserialize())
        .unwrap_or_else(|e| e.exit());
    
    if args.cmd_add {
        add(args.arg_url);
    } else if args.cmd_list {
        list();
    } else if args.cmd_poll {
        poll_all();
    } else if args.cmd_remove {
        remove(args.arg_pk);
    }
}

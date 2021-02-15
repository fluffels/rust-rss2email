extern crate docopt;
extern crate sendgrid;
extern crate serde;
extern crate sqlite;
extern crate xml;

use sendgrid::Mail;
use sendgrid::SGClient;
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

fn connect() -> sqlite::Connection {
    let conn = sqlite::open("db").unwrap();

    conn.execute("
        CREATE TABLE IF NOT EXISTS feeds (
            pk INTEGER PRIMARY KEY,
            url TEXT NOT NULL
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

fn poll(client: &SGClient, url: &str) {
    let resp = reqwest::blocking::get(url).unwrap();
    let body = resp.text().unwrap();
    let parser = EventReader::new(body.as_bytes());
    let mut title = String::new();
    let mut link = String::new();
    let mut desc = String::new();
    let mut el = String::new();
    for event in parser {
        match event {
            Ok(XmlEvent::StartElement { name, .. }) => {
                el = name.local_name;
            },
            Ok(XmlEvent::CData(text)) | Ok(XmlEvent::Characters(text)) => {
                if el == "title" {
                    title = text;
                } else if el == "description" {
                    desc = text;
                } else if el == "link" {
                    link = text;
                }
            },
            Ok(XmlEvent::EndElement { name }) => {
                if name.local_name == "item" {
                    let subject = format!("
                        [rss2email] {}
                    ", title);
                    let text = format!("
                        <div>
                            <h1>
                                <a href='{}'>{}</a>
                            </h1>
                            <p>
                                {}
                            </p>
                        </div>
                    ", link, title, desc);
                    let mail = Mail::new()
                        .add_from("jcwkroeze@pm.me")
                        .add_text(&text)
                        .add_subject(&subject)
                        .add_to(("jcwkroeze@pm.me", "Jan CW Kroeze").into());
                    client.send(mail).unwrap();
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

    let conn = connect();
    conn.iterate(sql, |pairs| {
        for &(_, value) in pairs.iter() {
            poll(&client, value.unwrap());
        }
        true
    }).unwrap();
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

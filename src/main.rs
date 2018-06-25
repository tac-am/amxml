//
// main.rs
//
extern crate amxml;

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::process;
use amxml::dom::*;

// =====================================================================
/// Sample application: reads the XML file and pretty print to stdout.
///
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} filename", &args[0]);
        process::exit(1);
    }
    let filename = args[1].clone();
    if let Err(e) = pretty_print(&filename) {
        eprintln!("Application error: {:?}", e);
        process::exit(1);
    }
}

fn pretty_print(filename: &str) -> Result<(), Box<Error>> {
    let mut fp = File::open(&filename)?;

    let mut xml_string = String::new();
    fp.read_to_string(&mut xml_string)?;

    let doc = new_document(&xml_string)?;

    println!("{}", doc.to_pretty_string());

    return Ok(());
}

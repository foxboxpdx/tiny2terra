extern crate tiny2terra;
extern crate serde;
extern crate serde_json;
extern crate clap;

use tiny2terra::types::*;
use tiny2terra::route53_parser;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::collections::HashMap;
use clap::{Arg, App};

// Main method uses Clap crate to build a fancy CLI.
// Should be simple enough to add the ability to use alternate parsers with
// alternate outputs should Route53 not be desired
fn main() {
    let matches = App::new("Tiny2Terra")
                          .version(clap::crate_version!())
                          .author("Fox Box <foxboxpdx@gmail.com>")
                          .about("Convert tinydns/djbdns to AWS TForm")
                          .arg(Arg::with_name("input")
                               .short("i")
                               .value_name("FILE")
                               .help("Specify tinydns file to read from")
                               .required(true)
                               .takes_value(true))
                          .arg(Arg::with_name("output")
                               .short("o")
                               .value_name("DIR")
                               .help("Output dir for TF files (default: ./terraform)")
                               .takes_value(true))
                          .arg(Arg::with_name("zone")
                               .short("z")
                               .value_name("ZONE")
                               .help("Destination R53 Zone name (defined in zones.tf)")
                               .required(true)
                               .takes_value(true))
                          .arg(Arg::with_name("lint")
                               .short("l")
                               .help("Validate input file only, do not output"))
                          .arg(Arg::with_name("stdout")
                               .short("s")
                               .help("Print output to STDOUT as well as file"))
                          .get_matches();

    // If no output dir specified, default to './terraform'
    let outdir = matches.value_of("output").unwrap_or("terraform");

    // As per Clap's documentation, it's safe to simply unwrap these since
    // they are marked as required(true).
    let infile = matches.value_of("input").unwrap();
    let zone   = matches.value_of("zone").unwrap();


    // If the -l flag is specified, we need to short-circuit pretty much the
    // entirety of main() and just run route53_parser::parse(), then exit.
    if matches.is_present("lint") {
        match route53_parser::parse(infile, zone) {
            Some(_) => {
                println!("Parsed {} ok!", infile);
                std::process::exit(0);
            },
            None => {
                println!("Problem parsing {}!", infile);
                std::process::exit(1);
            }
        };
    }

    // We'll need to get the base filename from 'infile' since 'outdir' is
    // relative to the directory we're executing from and stupid things will
    // happen otherwise.
    let basename = std::path::Path::new(infile).file_name().unwrap().to_str().unwrap();

    // Attempt to create our destination directory and file if it does not exist
    // No sense going through all the bother of parsing if we can't even output!
    let outfile = format!("{}/{}.tf", &outdir, &basename);
    match std::fs::create_dir_all(&outdir) {
        Ok(_) => {},
        Err(e) => {
            println!("Error creating directory {}: {}", &outdir, e);
            panic!("Bailing out");
        }
    };

    let ofile = match File::create(&outfile) {
        Ok(f) => f,
        Err(e) => {
            println!("Error creating output file {}: {}", &outfile, e);
            panic!("Bailing out");
        }
    };

    // Send the infile and zone data off to the processing function.  Returns
    // an Option<HashMap<String, Route53Record>>
    let parsed_hash = match route53_parser::parse(infile, zone) {
        Some(x) => x,
        None => {
            println!("Unable to parse file: {}", infile);
            panic!("Bailing out");
        }
    };

    // The parsed data is merely the inner HashMap of two required to build
    // a TerraFile struct.  Yes, it's ugly, JSON is like that sometimes.
    // Construct the outer hash, and build a TerraFile from the lot
    let mut outer_hash = HashMap::new();
    outer_hash.insert("aws_route53_record".to_string(), parsed_hash);
    let tform = TerraFile { resource: outer_hash };

    // Serialize it to a string using serde_json
    let outstring = match serde_json::to_string_pretty(&tform) {
        Ok(x) => x,
        Err(e) => {
            println!("Error serializing JSON: {}", e);
            panic!("Bailing out");
        }
    };

    // Dump the serialized JSON string to our output file
    let mut ofile_writer = BufWriter::new(ofile);
    match ofile_writer.write_all(outstring.as_bytes()) {
        Ok(_) => {},
        Err(e) => {
            println!("Error writing file {}: {}", &outfile, e);
            panic!("Bailing out");
        }
    }

    // Print to stdout if the -s flag was supplied
    if matches.is_present("stdout") {
        println!("{}", outstring);
    }

    // Complete
    println!("Successfully processed file: {}", infile);
}


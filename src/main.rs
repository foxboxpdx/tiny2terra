extern crate tiny2terra;
extern crate serde;
extern crate serde_json;
#[macro_use] extern crate clap;

use tiny2terra::types::*;
use tiny2terra::route53;
use tiny2terra::tinydns;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::collections::HashMap;
use clap::App;

// Main method uses Clap crate to build a fancy CLI from contents of cli.yml.
fn main() {
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).version(clap::crate_version!()).get_matches();

    // If no output dir specified, default to './terraform'
    let outdir = matches.value_of("output").unwrap_or("terraform");

    // As per Clap's documentation, it's safe to simply unwrap this since
    // it is marked as required:true
    let infile = matches.value_of("input").unwrap();

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
            println!("Bailing out");
            std::process::exit(1);
        }
    };

    let ofile = match File::create(&outfile) {
        Ok(f) => f,
        Err(e) => {
            println!("Error creating output file {}: {}", &outfile, e);
            println!("Bailing out");
            std::process::exit(1);
        }
    };

    // Subcommand: route53
    if let Some(r53_flags) = matches.subcommand_matches("route53") {
        // More required args to unwrap
        let fwdzone = r53_flags.value_of("fwdzone").unwrap();
        let ptrzone = r53_flags.value_of("ptrzone").unwrap();

        // Process the input file into a Vec of TinyDNSRecords
        let tdns_records = match tinydns::from_file(&infile) {
            Some(x) => x,
            None => {
                println!("Errors while parsing file: {}", infile);
                println!("Bailing out");
                std::process::exit(1);
            }
        };

        // Process the TinyDNSRecords into Route53Records
        let r53_records = match route53::generate(&fwdzone, &ptrzone, &tdns_records) {
            Some(x) => x,
            None => {
                println!("Errors while generating Route53 Records");
                println!("Bailing out");
                std::process::exit(1);
            }
        };

        // Create a wrapper hashmap for the R53 Records and make a serializable
        // struct for output
        let mut outer_hash = HashMap::new();
        outer_hash.insert("aws_route53_record".to_string(), r53_records);
        let r53_file = Route53File { resource: outer_hash };

        // Serialize it to a string using serde_json
        let outstring = match serde_json::to_string_pretty(&r53_file) {
            Ok(x) => x,
            Err(e) => {
                println!("Error serializing JSON: {}", e);
                std::process::exit(1);
            }
        };

        // If the -s flag was supplied, go ahead and print to STDOUT now
        if matches.is_present("stdout") {
            println!("{}", outstring);
        }

        // If we're just linting the file, exit now.
        if matches.is_present("lint") {
            println!("No errors detected while processing {}", infile);
            std::process::exit(0);
        }

        // Dump the serialized JSON string to our output file
        let mut ofile_writer = BufWriter::new(ofile);
        match ofile_writer.write_all(outstring.as_bytes()) {
            Ok(_) => {},
            Err(e) => {
                println!("Error writing file {}: {}", &outfile, e);
                std::process::exit(1);
            }
        }
    }

    // Complete
    println!("Successfully processed {} and wrote {}", infile, outfile);
}


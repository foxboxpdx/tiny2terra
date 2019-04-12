// Module providing functions to parse a TinyDNS input file
use std::fs::File;
use std::io::{BufReader, BufRead};
use std::collections::HashMap;
use types::Route53Record;

// Given an input filename and a Route53 Terraform zone name, attempt to
// read in and process TinyDNS-formatted records into Route53Record structs,
// collect these into a HashMap, and return.
pub fn parse(fname: &str, zone: &str) -> Option<HashMap<String, Route53Record>> {
    // Define our return hashmap
    let mut retval = HashMap::new();

    // Attempt to open and read file
    let f = match File::open(fname) {
        Ok(file) => file,
        Err(e) => {
            println!("Error opening file {}: {}", fname, e);
            return None;
        }
    };
    let reader = BufReader::new(&f);

    // Process each line, making this a separate function in case we
    // decide to change how we parse things.
    for line in reader.lines() {
        let l = line.expect("Couldn't get line?");
        let mut record = match parse_line(&l, &zone) {
            Some(x) => x,
            None => {
                println!("Error parsing line: {}", &l);
                return None;
            }
        };
        let mut record_name = format!("{}.{}", &record.rtype, &record.name);
        record_name = record_name.replace(".", "-").to_lowercase();

        // See if this record name exists in the return hashmap, and if so,
        // merge the two Route53Record structs
        if retval.contains_key(&record_name) {
            // We know this key exists so we can just use unwrap here
            let old_record = retval.remove(&record_name).unwrap();
            if !record.merge(&old_record) {
                println!("Error merging records:");
                println!("Old: {:?}\nNew: {:?}", old_record, record);
                return None;
            }
        }

        // Add this record to the hashmap
        retval.insert(record_name, record);
    }
    // Return the hashmap
    Some(retval)
}

// Given a string representing a single TinyDNS record, break it up and
// turn it into a Route53Record struct
pub fn parse_line(line: &str, zone: &str) -> Option<Route53Record> {
    // Line needs to have exactly two colons.
    let num_colons = line.matches(":").count();
    if num_colons != 2 {
        println!("Suspect line found: {}", line);
        return None;
    }
    // Split up the line by colon.  Using 'remove' here because Vec doesn't
    // implement shift() in Rust for some reason?
    let mut parts: Vec<&str> = line.split(':').collect();
    let pname = parts.remove(0);
    let rec   = parts.remove(0);
    let ttl   = parts.remove(0).parse::<i32>().unwrap_or(300);

    // Split off the prefix and the name
    let (prefix, name) = pname.split_at(1);

    // Match on the value of prefix to determine the record type.  Currently
    // only supprting A, PTR, CNAME, TXT; would have to rework this fn a 
    // bit to support additional record types.
    let rtype = match prefix {
        "+" => "A",
        "^" => "PTR",
        "C" => "CNAME",
        "'" => "TXT",
        _   => {
            println!("Unsupported prefix: {}", prefix);
            return None;
        }
    };

    // Remove any extraneous double-quotes from 'rec' or Terraform gets real
    // unhappy.
    let unquoted = rec.to_string().replace("\"", "");

    // Construct Zone ID from the supplied zone &str, convert . to -
    let zone_id = format!("${{aws_route53_zone.{}.zone_id}}",
                          &zone.to_string().replace(".", "-"));

    // Create the struct
    let record = Route53Record::new(&zone_id, name, rtype, &unquoted, ttl);

    // Return
    Some(record)
}

// Define structs and functions for generating Route53-specific Terraform
// output using TinyDNSRecords.
use std::collections::HashMap;
use types::{TinyDNSRecord, Route53Record};

// Given a Vec of TinyDNSRecords, a forward zone ID, and a ptr zone ID,
// construct a HashMap of Route53Records with names as the keys.
pub fn generate(fzone: &str, rzone: &str, tdrs: &Vec<TinyDNSRecord>) 
               -> Option<HashMap<String, Route53Record>> {
    // Init return hashmap
    let mut retval: HashMap<String, Route53Record> = HashMap::new();

    // Watch for any errors
    let mut error_flag = false;

    // Iterate through the tdrs vector, creating a single Route53Record each
    for rec in tdrs {
        // Decide which zone_id to set based on record type.  PTRs get the
        // reverse zone, everything else gets the forward zone.
        let zoneid = match rec.rtype.as_str() {
            "PTR" => &rzone,
            _     => &fzone
        };

        // Generate the record.
        let rec_vec = vec![rec.target.to_string()];
        let mut r53r = Route53Record {
            zone_id:  zoneid.to_string(),
            name:     rec.fqdn.to_string(),
            rtype:    rec.rtype.to_string(),
            records:  rec_vec,
            ttl:      rec.ttl
        };

        // Create a string representing a name to use as a hash key
        let mut record_name = format!("{}.{}", &r53r.rtype, &r53r.name);

        // If there's a period at the end we should get rid of it before 
        // converting the name into something AWS-compatable
        record_name = record_name.trim_end_matches('.').to_string();
        record_name = record_name.replace(".", "-").to_lowercase();

        // Check for an existing matching key in the hashmap and merge the
        // record structs if one is found
        if retval.contains_key(&record_name) {
            // Unwrap should be safe since we wouldn't be here otherwise
            let old_record = retval.remove(&record_name).unwrap();
            // If these are both PTR records, we got a problem here.
            if r53r.rtype.as_str() == "PTR" {
                println!("Error: Found two PTR records for the same IP!");
                println!("PTR: {}", &r53r.name);
                println!("FQDNs: {}, {}", &r53r.records[0], &old_record.records[0]);
                error_flag = true;
            } else {
                // Otherwise go ahead and try to merge them
                // This used to return None early but I want to try to process
                // everything in the files before bailing out in case there 
                // are multiple issues
                if !r53r.merge(&old_record) {
                    println!("Error merging records:");
                    println!("Old: {:?}\nNew: {:?}", old_record, r53r);
                    error_flag = true;
                }
            }
        }

        // Insert R53Record and name-key into the hashmap
        retval.insert(record_name, r53r);
    }

    // Return the hashmap if there were no errors, None otherwise
    match error_flag {
        true => None,
        false => Some(retval)
    }
}


// Define functions for processing TinyDNS flat files
use std::fs::File;
use std::io::{BufReader, BufRead};
use std::time::SystemTime;
use std::net::Ipv4Addr;
use types::TinyDNSRecord;

// Given a filename, read in the contents and generate a Vec of TDRs
pub fn from_file(fname: &str) -> Option<Vec<TinyDNSRecord>> {
    let mut retval = Vec::new();
    let mut error_flag = false;

    // Attempt to open and read file
    let f = match File::open(fname) {
        Ok(file) => file,
        Err(e) => {
            println!("Error opening file {}: {}", fname, e);
            return None;
        }
    };
    let reader = BufReader::new(&f);

    // Process each line in the file and call the appropriate parsing
    // function.  Remember that some prefixes generate more than one!
    // Because of that, all the parse_X functions return a vector that
    // can be simply append()-ed to retval. If there's an error, we simply
    // get back an empty vector.
    for line in reader.lines() {
        let l = line.expect("Couldn't get line?");
        let (prefix, data) = l.split_at(1);
        let mut parsed = match prefix {
            "+" => { parse("A", data) },
            "^" => { parse("PTR", data) },
            "C" => { parse("CNAME", data) },
            "'" => { parse("TXT", data) },
            "@" => { parse_mx(data) },
            "Z" => { parse_soa(data) },
            "." => { parse_anssoa(data) },
            "&" => { parse_ans(data) },
            "=" => { parse_aptr(data) },
            _   => {
                println!("Unsuported prefix: {}", prefix);
                Vec::new()
            }
        };
        if parsed.is_empty() {
            error_flag = true;
        } else {
            retval.append(&mut parsed);
        }
    }

    // Return the parsed records if there were no errors
    match error_flag {
        true => None,
        false => Some(retval)
    }
}

// Parse a basic DNS record into 1 TinyDNSRecord
// +fqdn:rec:ttl:timestamp:lo - A
// ^fqdn:rec:ttl:timestamp:lo - PTR
// Cfqdn:rec:ttl:timestamp:lo - CNAME
// 'fqdn:rec:ttl:timestamp:lo - TXT
pub fn parse(rtype: &str, data: &str) -> Vec<TinyDNSRecord> {
    // Create our return Vec
    let mut retval = Vec::new();

    // Split up the data by colon.
    let mut parts: Vec<&str> = data.split(':').collect();

    // The FQDN and Target are mandatory. Print an error and return an
    // empty Vec if there aren't at least 2 items in 'parts'
    if parts.len() < 2 {
        println!("Error parsing line: {} of type {}", data, rtype);
        return retval;
    }

    // Pull those parts out
    let fqdn = parts.remove(0);
    let rec = parts.remove(0);

    // Just in case there are extraneous quotes - Terraform gets angry
    // about those.
    let target = rec.to_string().replace("\"", "");

    // If this is an 'A' record, we should ensure 'rec' is a valid IPv4 addr
    if rtype == "A" {
        match rec.parse::<Ipv4Addr>() {
            Ok(_) => {},
            Err(e) => {
                println!("Error processing record: {}", data);
                println!("{}", e);
                return retval;
            }
        }
    }

    // See if there's a TTL in there since it would come next
    // Assign a default value of 300 if there's none provided
    // or if it can't be parsed as an i32.
    let ttl = match parts.is_empty() {
        true => 300,
        false => {
            parts.remove(0).parse::<i32>().unwrap_or(300)
        }
    };

    // Any data that may be left in 'parts' is extraneous and unneeded,
    // so proceed on to making a TDR, put it in retval, and return.
    let tdr = TinyDNSRecord {
        rtype: rtype.to_string(),
        fqdn:  fqdn.to_string(),
        target: target,
        ttl: ttl
    };
    retval.push(tdr);

    retval
}

// Parse an MX record into two TinyDNSRecords
// @fqdn:ip:x:dist:ttl:timestamp:lo
// (1) type=MX, fqdn=fqdn, target="dist x(.mx.fqdn)"
// (2) type=A,  fqdn=x(.mx.fqdn), target=ip
pub fn parse_mx(data: &str) -> Vec<TinyDNSRecord> {
    // Create return vec
    let mut retval = Vec::new();

    // Split up data by colon
    let mut parts: Vec<&str> = data.split(':').collect();

    // FQDN, target, mx_fqdn required; error and return on parts < 3
    if parts.len() < 3 {
        println!("Error parsing line: {} of type MX", data);
        return retval;
    }

    // Pull out required parts
    let fqdn = parts.remove(0);
    let ip = parts.remove(0);
    let x = parts.remove(0);

    // Make sure IP is an IP
    match ip.parse::<Ipv4Addr>() {
        Ok(_) => {},
        Err(e) => {
            println!("Error processing record: {}", data);
            println!("{}", e);
            return retval;
        }
    }

    // TinyDNS spec states that if x contains a period, it is used
    // as-is; otherwise, it becomes x.mx.fqdn.
    let mx_fqdn = match x.to_string().contains('.') {
        true => x.to_string(),
        false => format!("{}.mx.{}", x, fqdn)
    };

    // Do some fancy matching footwork to populate the mx_dist and ttl
    // depending on whether they were provided. Even though mx_dist will
    // wind up as part of a string, make sure it's a valid integer first.
    let (mx_dist, ttl) = match parts.len() {
        0 => (0, 300),
        1 => (parts.remove(0).parse::<i32>().unwrap_or(0), 300),
        _ => (parts.remove(0).parse::<i32>().unwrap_or(0),
              parts.remove(0).parse::<i32>().unwrap_or(300))
    };

    // Generate MX TDR
    let tdr1 = TinyDNSRecord {
        rtype:   "MX".to_string(),
        fqdn:    fqdn.to_string(),
        target:  format!("{} {}", mx_dist, mx_fqdn),
        ttl:     ttl
    };
    retval.push(tdr1);

    // Generate A TDR
    let tdr2 = TinyDNSRecord {
        rtype:  "A".to_string(),
        fqdn:   mx_fqdn,
        target: ip.to_string(),
        ttl:    ttl
    };
    retval.push(tdr2);

    // Return Vec
    retval
}

// Parse an SOA record 
// Zfqdn:ns:contact:serial:refresh:retry:expire:min:ttl:timestamp:lo
// serial, refresh, retry, expire, and min are optional and default to
// epoch, 16384, 2048, 1048576, and 2560.
pub fn parse_soa(data: &str) -> Vec<TinyDNSRecord> {
    // Create return vec
    let mut retval = Vec::new();

    // Split on colon
    let mut parts: Vec<&str> = data.split(':').collect();

    // Error and return if we don't have at least 3 items
    if parts.len() < 3 {
        println!("Error parsing line: {} of type SOA", data);
        return retval;
    }

    // Pull the required 3 off
    let fqdn    = parts.remove(0);
    let ns      = parts.remove(0);
    let contact = parts.remove(0);

    // As with MX, we can do some fancy footwork with match based on how
    // many items are left in the parts vector.  Start by getting an
    // epoch time in case we need it.
    let right_now = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_secs(),
        Err(_) => panic!("Something is REALLY wrong, SystemTime < EPOCH??")
    };

    // Now the match game. Again these wind up in a string but we want to
    // ensure they are valid integers first.
    let (ser, refr, retr, exp, min, ttl) = match parts.len() {
        0 => (right_now, 16384, 2048, 1048576, 2560, 300),
        1 => (parts.remove(0).parse::<u64>().unwrap_or(right_now),
              16384, 2048, 1048576, 2560, 300),
        2 => (parts.remove(0).parse::<u64>().unwrap_or(right_now),
              parts.remove(0).parse::<i32>().unwrap_or(16384),
              2048, 1048576, 2560, 300),
        3 => (parts.remove(0).parse::<u64>().unwrap_or(right_now),
              parts.remove(0).parse::<i32>().unwrap_or(16384),
              parts.remove(0).parse::<i32>().unwrap_or(2048),
              1048576, 2560, 300),
        4 => (parts.remove(0).parse::<u64>().unwrap_or(right_now),
              parts.remove(0).parse::<i32>().unwrap_or(16384),
              parts.remove(0).parse::<i32>().unwrap_or(2048),
              parts.remove(0).parse::<i32>().unwrap_or(1048576),
              2560, 300),
        5 => (parts.remove(0).parse::<u64>().unwrap_or(right_now),
              parts.remove(0).parse::<i32>().unwrap_or(16384),
              parts.remove(0).parse::<i32>().unwrap_or(2048),
              parts.remove(0).parse::<i32>().unwrap_or(1048576),
              parts.remove(0).parse::<i32>().unwrap_or(2560), 300),
        _ => (parts.remove(0).parse::<u64>().unwrap_or(right_now),
              parts.remove(0).parse::<i32>().unwrap_or(16384),
              parts.remove(0).parse::<i32>().unwrap_or(2048),
              parts.remove(0).parse::<i32>().unwrap_or(1048576),
              parts.remove(0).parse::<i32>().unwrap_or(2560),
              parts.remove(0).parse::<i32>().unwrap_or(300))
    };
    // That could probably be a lot cleaner.  Oh well.

    // Generate that target string
    let target = format!("{} {} {} {} {} {} {}", ns, contact, ser, refr, 
                         retr, exp, min);

    // Generate TDR, push, return
    let tdr = TinyDNSRecord {
        rtype:  "SOA".to_string(),
        fqdn:   fqdn.to_string(),
        target: target,
        ttl:    ttl
    };
    retval.push(tdr);

    // Return
    retval
}

// Parse a combination A/NS/SOA record into 3 TinyDNSRecords
// .fqdn:ip:x:ttl:timestamp:lo
// (1) type=NS, fqdn=x(.ns.fqdn), target=fqdn
// (2) type=A,  fqdn=x(.ns.fqdn), target=ip
// (3) type=SOA fqdn=fqdn, target="x hostmaster.fqdn default-values"
pub fn parse_anssoa(data: &str) -> Vec<TinyDNSRecord> {
    // Create return vec
    let mut retval = Vec::new();

    // Split on colon
    let mut parts: Vec<&str> = data.split(':').collect();

    // Make sure there's enough pieces
    if parts.len() < 3 {
        println!("Error parsing line: {} of type A/NS/SOA", data);
        return retval;
    }

    // Get 'em
    let fqdn = parts.remove(0);
    let ip = parts.remove(0); // This can be empty
    let x = parts.remove(0);

    // Make sure IP is an IP
    match ip.parse::<Ipv4Addr>() {
        Ok(_) => {},
        Err(e) => {
            println!("Error processing record: {}", data);
            println!("{}", e);
            return retval;
        }
    }

    // Thankfully there's no big ugly match chains here, just a boolean
    let ttl = match parts.is_empty() {
        true => 300,
        false => parts.remove(0).parse::<i32>().unwrap_or(300)
    };

    // As with MX, if x contains a period, it is used as is; otherwise, it
    // becomes x.ns.fqdn.
    let ns_fqdn = match x.to_string().contains('.') {
        true => x.to_string(),
        false => format!("{}.ns.{}", x, fqdn)
    };

    // Start building TDRs. If ip is empty, don't create (2).
    let tdr1 = TinyDNSRecord {
        rtype:  "NS".to_string(),
        fqdn:   ns_fqdn.to_string(),
        target: fqdn.to_string(),
        ttl:    ttl
    };
    retval.push(tdr1);

    if !ip.is_empty() {
        let tdr2 = TinyDNSRecord {
            rtype:  "A".to_string(),
            fqdn:   ns_fqdn.to_string(),
            target: ip.to_string(),
            ttl:    ttl
        };
        retval.push(tdr2);
    }

    let target = format!("{} hostmaster.{} 1 1 1 1 60", &ns_fqdn, &fqdn);
    let tdr3 = TinyDNSRecord {
        rtype:  "SOA".to_string(),
        fqdn:   fqdn.to_string(),
        target: target,
        ttl:    ttl
    };
    retval.push(tdr3);

    // Return
    retval
}

// Parse a combination A/NS record into 2 TinyDNSRecords
// &fqdn:ip:x:ttl:timestamp:lo
// (1) type=NS, fqdn=x(.ns.fqdn), target=fqdn
// (2) type=A,  fqdn=x(.ns.fqdn), target=ip
pub fn parse_ans(data: &str) -> Vec<TinyDNSRecord> {
    // Create return vec
    let mut retval = Vec::new();

    // Split on colon
    let mut parts: Vec<&str> = data.split(':').collect();

    // 3 shall be the number of the counting
    if parts.len() < 3 {
        println!("Error parsing line: {} of type A/NS", data);
        return retval;
    }

    // You're gonna extract HIM?
    let fqdn = parts.remove(0);
    let ip = parts.remove(0);
    let x = parts.remove(0);

    // Make sure IP is an IP
    match ip.parse::<Ipv4Addr>() {
        Ok(_) => {},
        Err(e) => {
            println!("Error processing record: {}", data);
            println!("{}", e);
            return retval;
        }
    }

    // Check for TTL
    let ttl = match parts.is_empty() {
        true => 300,
        false => parts.remove(0).parse::<i32>().unwrap_or(300)
    };

    // Check x for dots
    let ns_fqdn = match x.to_string().contains('.') {
        true => x.to_string(),
        false => format!("{}.ns.{}", x, fqdn)
    };

    // Build TDRs
    let tdr1 = TinyDNSRecord {
        rtype:  "NS".to_string(),
        fqdn:   ns_fqdn.to_string(),
        target: fqdn.to_string(),
        ttl:    ttl
    };
    retval.push(tdr1);

    let tdr2 = TinyDNSRecord {
        rtype:  "A".to_string(),
        fqdn:   ns_fqdn.to_string(),
        target: ip.to_string(),
        ttl:    ttl
    };
    retval.push(tdr2);

    // Return
    retval
}

// Parse a combination A/PTR record into 2 TinyDNSRecords
// =fqdn:ip:ttl:timestamp:lo
// (1) type=A, fqdn=fqdn, target=ip
// (2) type=PTR, fqdn=arpaized-ip, target=fqdn
pub fn parse_aptr(data: &str) -> Vec<TinyDNSRecord> {
    // Create return vec
    let mut retval = Vec::new();

    // Split on colon
    let mut parts: Vec<&str> = data.split(':').collect();

    // It takes two to tango
    if parts.len() < 2 {
        println!("Error parsing line: {} of type A/PTR", data);
        return retval;
    }

    // Front and back
    let fqdn = parts.remove(0);
    let ip = parts.remove(0);

    // Make sure IP is an IP
    match ip.parse::<Ipv4Addr>() {
        Ok(_) => {},
        Err(e) => {
            println!("Error processing record: {}", data);
            println!("{}", e);
            return retval;
        }
    };

    // TTL check
    let ttl = match parts.is_empty() {
        true => 300,
        false => parts.remove(0).parse::<i32>().unwrap_or(300)
    };

    // Build a PTR FQDN from the IP
    let mut ipbits: Vec<&str> = ip.split('.').collect();
    ipbits.reverse();
    let backwards = ipbits.join(".");
    let ptr_fqdn = format!("{}.in-addr.arpa", backwards);

    // Build TDRs
    let tdr1 = TinyDNSRecord {
        rtype:  "A".to_string(),
        fqdn:   fqdn.to_string(),
        target: ip.to_string(),
        ttl:    ttl
    };
    retval.push(tdr1);

    let tdr2 = TinyDNSRecord {
        rtype:  "PTR".to_string(),
        fqdn:   ptr_fqdn,
        target: fqdn.to_string(),
        ttl:    ttl
    };
    retval.push(tdr2);

    // Return
    retval
}

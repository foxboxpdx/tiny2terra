#[macro_use] extern crate serde_derive;
extern crate serde;
extern crate serde_json;

pub mod types;
pub mod route53;
pub mod tinydns;

// Gettin' testy with it
#[cfg(test)]
mod tests {
    use types::*;
    use std::collections::HashMap;
    use route53_parser;

    // Make sure the 'new' function properly converts the &strs sent to it 
    // into Strings, and the record &str into a Vec<String>.
    #[test]
    fn test_record_new() {
        let rec = vec!["a".to_string()];
        let x = Route53Record { 
            zone_id: "zone".to_string(),
            name:    "name".to_string(), 
            rtype:   "rtype".to_string(), 
            records: rec, 
            ttl:     123 };
        let y = Route53Record::new("zone", "name", "rtype", "a", 123);
        assert_eq!(x, y);
    }

    // Make sure add_record properly adds a &str to the records Vec
    #[test]
    fn test_record_add() {
        let rec = vec!["a".to_string(), "b".to_string()];
        let x = Route53Record {
            zone_id: "zone".to_string(),
            name:    "name".to_string(),
            rtype:   "rtype".to_string(),
            records: rec,
            ttl:     123 };
        let mut y = Route53Record::new("zone", "name", "rtype", "a", 123);
        y.add_record("b");
        assert_eq!(x, y);
    }

    // Make sure merge properly merges together the records vectors of the two
    // structs involved
    #[test]
    fn test_record_merge() {
        let rec = vec!["a".to_string(), "b".to_string()];
        let x = Route53Record {
            zone_id: "zone".to_string(),
            name:    "name".to_string(),
            rtype:   "rtype".to_string(),
            records: rec,
            ttl:     123 };
        let mut y = Route53Record::new("zone", "name", "rtype", "a", 123);
        let z = Route53Record::new("zone", "name", "rtype", "b", 123);
        y.merge(&z);
        assert_eq!(x, y);
    }

    // Test equality
    #[test]
    fn test_record_eq() {
        let x = Route53Record::new("zone", "name", "rtype", "a", 123);
        let y = Route53Record::new("zone", "name", "rtype", "a", 123);
        let z = Route53Record::new("zone", "name", "rtype", "b", 123);
        let a = Route53Record::new("zone", "eman", "rtype", "a", 123);
        assert_eq!(x, y);
        assert_ne!(x, z);
        assert_ne!(x, a);
    }

    // route53_parser tests
    // Ensure parse_line turns a valid line into a valid Route53Record
    #[test]
    fn test_r53_parse_line() {
        let line = "+test.foo.com:1.2.3.4:450";
        let zone = "${aws_route53_zone.zone.zone_id}";
        let x = Route53Record::new(&zone, "test.foo.com", "A", "1.2.3.4", 450);
        let y = route53_parser::parse_line(&line, "zone").unwrap();
        assert_eq!(x, y);
    }

    // Ensure a return of None on unsupported prefix
    #[test]
    fn test_r53_parse_line_bad_prefix() {
        let x = route53_parser::parse_line("@foo.com:1.2.3.4:123", "zone");
        assert_eq!(x.is_none(), true);
    }

    // Ensure a return of None on invalid line (doesn't split on : into 3)
    #[test]
    fn test_r53_parse_line_bad_line() {
        let x = route53_parser::parse_line("foo", "zone");
        assert_eq!(x.is_none(), true);
    }

    // Ensure non-integer TTL is ignored and replaced by 300
    #[test]
    fn test_r53_parse_line_bad_ttl() {
        let line = "+test.foo.com:1.2.3.4:foo";
        let y = route53_parser::parse_line(&line, "zone").unwrap();
        assert_eq!(300, y.ttl);
    }

    // Make sure the 4 accepted prefixes are all accepted
    #[test]
    fn test_r53_parse_line_check_prefixes() {
        let a   = route53_parser::parse_line("+test.foo.com:1.2.3.4:300", "z");
        let ptr = route53_parser::parse_line("^4.3.2.1.in-addr.arpa:foo:300", "z");
        let cn  = route53_parser::parse_line("Cbar:foo:300", "z");
        let txt = route53_parser::parse_line("'baz:string:300", "z");
        assert_eq!(a.is_some(), true);
        assert_eq!(ptr.is_some(), true);
        assert_eq!(cn.is_some(), true);
        assert_eq!(txt.is_some(), true);
    }

    // Ensure a bad/missing filename returns a None
    #[test]
    fn test_r53_parse_parse_bad_file() {
        let x = route53_parser::parse("foo", "foo");
        assert_eq!(x.is_none(), true);
    }

    // Ensure a file with bad lines returns None
    #[test]
    fn test_r53_parse_parse_bad_line() {
        let x = route53_parser::parse("baddata", "foo");
        assert_eq!(x.is_none(), true);
    }

    // Ensure a good file parses and matches known good hashmap
    #[test]
    fn test_r53_parse_parse_good_file() {
        let zone = "${{aws_route53_zone.foo.zone_id}}";
        let a = Route53Record::new(&zone, "foo.nike.com", "A", "1.2.3.4", 600);
        let b = Route53Record::new(&zone, "4.3.2.1.in-addr.arpa", "PTR", "foo.nike.com", 600);
        let c = Route53Record::new(&zone, "bar.nike.com", "CNAME", "foo.nike.com", 600);
        let d = Route53Record::new(&zone, "txt.nike.com", "TXT", "Some text string", 600);
        let mut good_hash = HashMap::new();
        good_hash.insert("a-foo-nike-com".to_string(), a);
        good_hash.insert("ptr-4-3-2-1-in-addr-arpa".to_string(), b);
        good_hash.insert("cname-bar-nike-com".to_string(), c);
        good_hash.insert("txt-txt-nike-com".to_string(), d);
        let x = route53_parser::parse("testdata", "foo").unwrap();
        assert_eq!(good_hash.len(), x.len());
    }
}


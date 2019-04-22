use std::collections::HashMap;

#[derive(Serialize)]
pub struct Route53File {
  pub resource: HashMap<String, HashMap<String, Route53Record>>
}

#[derive(Serialize, Debug)]
pub struct Route53Record {
  pub zone_id: String,
  pub name: String,
  #[serde(rename="type")]
  pub rtype: String,
  pub records: Vec<String>,
  pub ttl: i32
}

#[derive(Debug)]
pub struct TinyDNSRecord {
    pub rtype: String,
    pub fqdn: String,
    pub target: String,
    pub ttl: i32,
}

impl Route53Record {
    // Merge the records vectors of this and another struct
    // Return false if the record types are mismatched or there's
    // any other sorts of issues with the merge
    pub fn merge(&mut self, other: &Self) -> bool {
        if self.rtype != other.rtype {
            return false;
        }
        let mut newvec = other.records.clone();
        newvec.append(&mut self.records.clone());
        self.records = newvec;
        true
    }
}

impl Eq for Route53Record {}

impl PartialEq for Route53Record {
    // We need to sort the records vectors before testing equality, but we
    // don't want to have to take mutable borrows here, so compare copies of
    // the two vectors.  Not the most efficient but these vectors aren't
    // likely to be more than a couple elements long.
    fn eq(&self, other: &Self) -> bool {
        // For some reason I can't call clone.sort without getting back
        // an empty tuple which really screws everything up.
        let mut my_records = self.records.clone();
        my_records.sort();
        let mut other_records = other.records.clone();
        other_records.sort();
        self.name == other.name && 
            my_records   == other_records &&
            self.zone_id == other.zone_id &&
            self.rtype   == other.rtype &&
            self.ttl     == other.ttl
    }
}


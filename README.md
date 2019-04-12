# tiny2terra

Convert TinyDNS/DJBDNS config files into Terraform Route53 config files.

## Usage
    tiny2terra -i <FILE> -z <ZONE> [ -o <DIR> ] [ -s ] [ -l ]

## Options
    -i <FILE> - Input file in TinyDNS format to read from
    -z <ZONE> - Name of destination Route53 DNS Zone (dots will be converted to hyphens)
    -o <DIR>  - (Optional) Output directory for Terraform file (default: terraform)
    -s        - (Optional) Print TF JSON to STDOUT as well as to file
    -l        - (Optional) Lint input file only, do not output Terraform

## Supported Record Types
* 'A' - Use `+` as a prefix
* 'CNAME' - Use `C` as a prefix
* 'TXT' - Use `'` as a prefix
* 'PTR' - Use `^` as a prefix

## Example Input
```
+foo.example.com:10.0.0.1:900
Cbar.example.com:foo.example.com.:900
'foo.example.com:Ascii text string:86400
^1.0.0.10.in-addr.arpa:foo.example.com:900
```

## Example Output
```json
{
  "resource": {
    "aws_route53_record": {
      "txt-foo-example-com": {
        "zone_id": "${{aws_route53_zone.example-com.zone_id}}",
        "name": "foo.example.com",
        "type": "TXT",
        "records": [
          "Ascii text string"
        ],
        "ttl": 86400
      },
      "a-foo-example-com": {
        "zone_id": "${{aws_route53_zone.example-com.zone_id}}",
        "name": "foo.example.com",
        "type": "A",
        "records": [
          "10.0.0.1"
        ],
        "ttl": 900
      },
      "ptr-1-0-0-10-in-addr-arpa": {
        "zone_id": "${{aws_route53_zone.example-com.zone_id}}",
        "name": "1.0.0.10.in-addr.arpa",
        "type": "PTR",
        "records": [
          "foo.example.com"
        ],
        "ttl": 900
      },
      "cname-bar-example-com": {
        "zone_id": "${{aws_route53_zone.example-com.zone_id}}",
        "name": "bar.example.com",
        "type": "CNAME",
        "records": [
          "foo.example.com."
        ],
        "ttl": 900
      }
    }
  }
}
```

## Notes
* Zones and providers should be defined in their own Terraform file(s).  The value
of the `-z` flag should match up with whatever 'friendly' name the destination zone
has been given in its TF file.  Example:

```
resource "aws_route53_zone" "example-com" {
      name = "example.com"
}
```

* If zones/records already exist in AWS Route53, you will need to import them
into Terraform before they can be properly managed.  This is best left as an
exercise to the reader.  (It's not difficult, just tedious.)
* If using this program as part of an automated build, be sure to run a quick 
`terraform validate` on the produced file(s).  While this program will output
JSON which is valid Terraform formatted, it makes no guarantees the data will 
therefore be accepted by Route53.  Garbage in, garbage out - typos and bad input
will be carried through.

### Requirements:
Rust 1.22.1+


##### Credits
Original Concept: djbdns2terraform (https://github.com/dpetzold/djbdns2terraform.git)

Rust translation FoxBox <foxboxpdx@gmail.com>

##### Version
0.4.11 11-April-2019



use base64::{engine::general_purpose, Engine as _};
use bytes::Bytes;
use memchr::{memchr, memchr_iter};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use secrecy::SecretString;
use std::collections::HashMap;

pub fn generate_random_secret() -> SecretString {
    let rand_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(30)
        .map(char::from)
        .collect();

    rand_string.into()
}

pub fn base64_encode(original: Vec<u8>) -> String {
    general_purpose::STANDARD_NO_PAD.encode(original)
}

pub fn base64_decode(encoded: String) -> Result<Vec<u8>, anyhow::Error> {
    Ok(general_purpose::STANDARD_NO_PAD.decode(encoded)?)
}

pub fn parse_query_string(query: &Bytes) -> Result<HashMap<String, String>, anyhow::Error> {
    let mut map = HashMap::new();
    if query.is_empty() {
        return Ok(map);
    }
    //Each entry is split with &
    let mut and_iter = memchr_iter(b'&', query);
    let mut next_slice_start_at = 0;
    let mut data_left = true;
    while data_left {
        let slice;
        match and_iter.next() {
            Some(next_and) => {
                slice = query.slice(next_slice_start_at..next_and);
                next_slice_start_at = next_and + 1;
            }
            None => {
                //we are at the end
                slice = query.slice(next_slice_start_at..);
                data_left = false;
            }
        }
        //each entry is key value pair split by =
        if let Some(equal_i) = memchr(b'=', &slice) {
            let key = String::from_utf8_lossy(&slice.slice(..equal_i)).to_string();
            let value = String::from_utf8_lossy(&slice.slice(equal_i + 1..)).to_string();
            map.insert(key, value);
        } else {
            return Err(anyhow::Error::msg("Invalid entry"));
        }
    }
    Ok(map)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_string() {
        let query_string = "flavor=vanilla&food=ice cream";
        let query_bytes = Bytes::from_static(query_string.as_bytes());
        let query = parse_query_string(&query_bytes).expect("Error Parsing Query String");
        assert_eq!(
            "vanilla",
            query.get("flavor").expect("Flavor Missing in Map")
        );
        assert_eq!("ice cream", query.get("food").expect("Food Misisng in map"));
    }
}

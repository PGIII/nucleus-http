use rand::{distributions::Alphanumeric, thread_rng, Rng};
use secrecy::SecretString;
use base64::{Engine as _, engine::general_purpose, alphabet::Alphabet};

pub fn generate_random_secret() -> SecretString {
    let rand_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(30)
        .map(char::from)
        .collect();

    rand_string.into()
}

pub fn base64_encode(original: Vec<u8>) -> String {
    let encoded = general_purpose::STANDARD_NO_PAD.encode(original);
    encoded
}

pub fn base64_decode(encoded: String) -> Result<Vec<u8>, anyhow::Error> {
    Ok(general_purpose::STANDARD_NO_PAD.decode(encoded)?)
}

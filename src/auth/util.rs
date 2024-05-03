use sha256::digest;

pub fn validate_password(plain: &str, hashed: &str) -> Result<(), () > {
    let hash = digest(plain);
    if hashed == hash {Ok(())} else { Err(()) }
}

pub fn generate_password(plain: &str) -> String {
    let hash = digest(plain);
    return hash
}
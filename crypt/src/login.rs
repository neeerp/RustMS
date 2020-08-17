use bcrypt::{hash, verify, BcryptResult, DEFAULT_COST};

pub fn hash_password(pw: &str) -> BcryptResult<String> {
    hash(pw, DEFAULT_COST)
}

pub fn validate_against_hash(plain: &str, hashed: &str) -> BcryptResult<bool> {
    verify(plain, hashed)
}

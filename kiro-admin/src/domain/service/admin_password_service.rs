use anyhow::Result;

pub trait AdminPasswordService: Send + Sync {
    #[allow(dead_code)]
    fn hash_password(&self, password: &str) -> Result<String>;

    fn verify_password(&self, password: &str, password_hash: &str) -> Result<bool>;
}

pub trait InternalTokenFuzzer: Send + Sync {
    fn match_closest(&self, s: &String) -> Result<String, String>;
}

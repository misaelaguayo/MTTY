pub trait Frontend {
    fn r#type(&mut self, text: &str);
    fn poll_event(&mut self);
}

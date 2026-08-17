pub trait Document {
    fn id(&self) -> String;
    fn fields(&self) -> Vec<Field>;
}

pub struct Field {
    pub name: String,
    pub text: String,
}

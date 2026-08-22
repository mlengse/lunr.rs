pub trait Document {
    fn id(&self) -> String;
    fn fields(&self) -> Vec<Field>;
    fn boost(&self) -> f64 { 1.0 }
}

pub struct Field {
    pub name: String,
    pub text: String,
    pub boost: f64,
}

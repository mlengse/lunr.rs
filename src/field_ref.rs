use serde::ser::{Serialize, Serializer};

const JOINER: &str = "/";

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub struct FieldRef {
    pub document_ref: String,
    pub field_name: String,
}

impl FieldRef {
    pub fn new<S: Into<String>>(document_ref: S, field_name: S) -> FieldRef {
        FieldRef {
            document_ref: document_ref.into(),
            field_name: field_name.into(),
        }
    }

    pub fn from_string(field_ref_string: &str) -> Option<FieldRef> {
        let parts: Vec<&str> = field_ref_string.splitn(2, JOINER).collect();
        if parts.len() != 2 {
            return None;
        }
        Some(FieldRef::new(parts[1], parts[0]))
    }

    pub fn to_ref_string(&self) -> String {
        format!("{}{}{}", self.field_name, JOINER, self.document_ref)
    }
}

impl std::fmt::Display for FieldRef {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}{}{}", self.field_name, JOINER, self.document_ref)
    }
}


impl Serialize for FieldRef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        serializer.serialize_str(&format!("{}{}{}", self.field_name, JOINER, self.document_ref))
    }
}

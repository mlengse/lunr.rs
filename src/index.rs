use crate::inverted_index::InvertedIndex;
use crate::field_ref::FieldRef;
use crate::vector::Vector;
use crate::builder::Builder;

use serde::ser::{Serialize, Serializer, SerializeStruct};

type FieldVector = (FieldRef, Vector);

pub struct Index {
    version: String,
    inverted_index: InvertedIndex,
    field_vectors: Vec<FieldVector>,
    fields: Vec<String>,
    pipeline: Vec<String>,
}

impl From<Builder> for Index {
    fn from(mut builder: Builder) -> Index {
        builder.build();

        Index {
            version: String::from("2.3.9"),
            pipeline: vec![],
            inverted_index: builder.inverted_index,
            field_vectors: builder.field_vectors
                .into_iter()
                .collect(),
            fields: builder.fields,
        }
    }
}

impl Serialize for Index {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        let mut index = serializer.serialize_struct("Index", 5)?;

        index.serialize_field("version", &self.version)?;
        index.serialize_field("pipeline", &self.pipeline)?;
        index.serialize_field("fields", &self.fields)?;
        index.serialize_field("fieldVectors", &self.field_vectors)?;
        index.serialize_field("invertedIndex", &self.inverted_index)?;

        index.end()
    }
}

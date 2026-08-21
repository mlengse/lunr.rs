mod field_ref;
mod token;
pub mod builder;
pub mod document;
mod inverted_index;
mod vector;
pub mod index;

pub mod idf;
pub mod match_data;
pub mod pipeline;
pub mod set;
pub mod stemmer;
pub mod stop_word_filter;
pub mod token_set;
pub mod trimmer;
pub mod query;
pub mod query_lexer;
pub mod query_parse_error;
pub mod query_parser;
pub mod utils;

use builder::Builder;
use index::Index;
use pipeline::PipelineFunction;

pub fn lunr(config: impl FnOnce(&mut Builder)) -> Index {
    let mut builder = Builder::default();

    builder.pipeline.add_function(PipelineFunction::Trimmer);
    builder.pipeline.add_function(PipelineFunction::StopWordFilter);
    builder.pipeline.add_function(PipelineFunction::Stemmer);

    builder.search_pipeline.add_function(PipelineFunction::Stemmer);

    config(&mut builder);

    Index::from(builder)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}

use error::Error;
use resources::parse::jmdict::languages::Language;
use vector_space_model::DocumentVector;

use crate::engine::{
    document::MultiDocument,
    result::{ResultItem, SearchResult},
    simple_gen_doc::GenDoc,
    FindExt,
};

use self::index::Index;

pub(crate) mod index;

pub(crate) struct Find<'a> {
    limit: usize,
    offset: usize,
    query: &'a str,
}

impl<'a> FindExt for Find<'a> {
    type ResultItem = ResultItem;
    type GenDoc = GenDoc;
    type Document = MultiDocument;

    #[inline]
    fn get_limit(&self) -> usize {
        self.limit
    }

    #[inline]
    fn get_offset(&self) -> usize {
        self.offset
    }

    #[inline]
    fn get_query_str(&self) -> &str {
        &self.query
    }
}

impl<'a> Find<'a> {
    #[inline]
    pub(crate) fn new(query: &'a str, limit: usize, offset: usize) -> Self {
        Self {
            limit,
            offset,
            query,
        }
    }

    /// Do a foreign word search
    pub(crate) async fn find(&self) -> Result<SearchResult, Error> {
        let index = index::INDEX.get().ok_or(Error::Unexpected)?;

        let query_vec = match self.gen_query(&index) {
            Some(query) => query,
            None => return Ok(SearchResult::default()),
        };

        self.find_by_vec(query_vec).await
    }

    /// Do a foreign word search with a custom `query_vec`
    pub(crate) async fn find_by_vec(
        &self,
        query_vec: DocumentVector<GenDoc>,
    ) -> Result<SearchResult, Error> {
        let index = index::INDEX.get().ok_or(Error::Unexpected)?;

        // VecStore is surrounded by an Arc
        let mut doc_store = index.get_vector_store().clone();

        // All vectors in queries dimensions
        let dimensions = query_vec.vector().vec_indices().collect::<Vec<_>>();

        // Retrieve all matching vectors
        let document_vectors = doc_store
            .get_all_async(&dimensions)
            .await
            .map_err(|_| error::Error::NotFound)?;

        let result = self
            .vecs_to_result_items(&query_vec, &document_vectors, 0f32)
            .into_iter()
            .map(|i| {
                let rel = i.relevance;
                i.document.seq_ids.iter().map(move |j| (*j, rel))
            })
            .flatten()
            .map(|(seq_id, rel)| ResultItem {
                seq_id,
                relevance: rel,
                language: Language::English,
            })
            .collect();

        Ok(SearchResult::new(result))
    }

    /// Generate a document vector out of `query_str`
    fn gen_query(&self, index: &Index) -> Option<DocumentVector<GenDoc>> {
        let query = self
            .fixed_term(index)
            .unwrap_or(self.get_query_str())
            .to_string();

        let query_document = GenDoc::new(vec![query]);
        DocumentVector::new(index.get_indexer(), query_document.clone())
    }

    /// Returns Some(&str) with an alternative search-term in case original query does not exist as
    /// term. None if no alternative term was found, there was no tree loaded or the query is
    /// already in term list
    fn fixed_term(&self, index: &Index) -> Option<&str> {
        let query_str = self.get_query_str();

        let mut indexer = index.get_indexer().clone();

        let has_term = indexer.find_term(&query_str).is_some()
            || indexer.find_term(&query_str.to_lowercase()).is_some();

        if has_term {
            return None;
        }

        let mut res = index::get_term_tree().find(&query_str.to_string(), 1);
        if res.is_empty() {
            res = index::get_term_tree().find(&query_str.to_string(), 2);
        }
        res.sort_by(|a, b| a.1.cmp(&b.1));
        res.get(0).map(|i| i.0.as_str())
    }
}

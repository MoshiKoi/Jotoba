use super::{
    result::word::{Item, Reading},
    search::{Search, SearchMode},
};
use crate::{
    error::Error,
    models::{dict::Dict, sense},
    parse::jmdict::{information::Information, languages::Language, priority::Priority},
    DbPool,
};

use diesel::prelude::*;
use itertools::Itertools;
use tokio_diesel::*;

#[derive(Clone)]
pub struct WordSearch<'a> {
    search: Search<'a>,
    db: &'a DbPool,
    language: Option<Language>,
}

impl<'a> WordSearch<'a> {
    pub fn new(db: &'a DbPool, query: &'a str) -> Self {
        Self {
            search: Search::new(query, SearchMode::Variable),
            db,
            language: None,
        }
    }
    /// Use a specific language for the search
    pub fn with_language(&mut self, language: Language) -> &mut Self {
        self.language = Some(language);
        self
    }

    /// Use a specific mode for the search
    pub fn with_mode(&mut self, mode: SearchMode) -> &mut Self {
        self.search.mode = mode;
        self
    }

    /// Use a specific limit for the search
    pub fn with_limit(&mut self, limit: u16) -> &mut Self {
        self.search.limit = limit;
        self
    }

    /// Searches a native word
    pub async fn search_native(&mut self) -> Result<Vec<Item>, Error> {
        // Load sequence ids to display
        let seq_ids: Vec<i32> = self.get_sequence_ids_by_native().await?;

        self.get_results(&seq_ids).await
    }

    async fn get_results(&self, seq_ids: &Vec<i32>) -> Result<Vec<Item>, Error> {
        // Request Redings and Senses in parallel
        let (word_items, senses): (Vec<Item>, Vec<sense::Sense>) =
            futures::try_join!(self.load_readings(&seq_ids), self.load_senses(&seq_ids))?;

        Ok(Self::merge_words_with_senses(word_items, senses))
    }

    fn merge_words_with_senses(word_items: Vec<Item>, senses: Vec<sense::Sense>) -> Vec<Item> {
        // Map result into a usable word::Item an return it
        word_items
            .into_iter()
            .map(|mut word| {
                word.senses = senses
                    .iter()
                    .filter(|i| i.sequence == word.sequence)
                    .cloned()
                    .into_iter()
                    // Create a Vec<Sense> grouped by the gloss position
                    .group_by(|i| i.gloss_pos)
                    .into_iter()
                    .map(|(_, j)| j.collect_vec().into())
                    .collect_vec();

                word
            })
            .collect_vec()
    }

    /// Find the sequence ids of the results to load
    async fn get_sequence_ids_by_foreign(&mut self) -> Result<Vec<i32>, Error> {
        use crate::schema::dict::dsl::*;

        let predicate = {
            match self.search.mode {
                SearchMode::Exact => reading.like(self.search.query.to_owned()),
                SearchMode::Variable => reading.like(format!("%{}%", self.search.query)),
                SearchMode::LeftVariable => reading.like(format!("%{}", self.search.query)),
                SearchMode::RightVariable => reading.like(format!("{}%", self.search.query)),
            }
        };

        // Wait for tokio-diesel to support boxed queries #20
        if self.search.limit > 0 {
            Ok(dict
                .select(sequence)
                .filter(predicate)
                .limit(self.search.limit as i64)
                .get_results_async(&self.db)
                .await?)
        } else {
            Ok(dict
                .select(sequence)
                .filter(predicate)
                .get_results_async(&self.db)
                .await?)
        }
    }

    /// Find the sequence ids of the results to load
    async fn get_sequence_ids_by_native(&mut self) -> Result<Vec<i32>, Error> {
        use crate::schema::dict::dsl::*;

        let predicate = {
            match self.search.mode {
                SearchMode::Exact => reading.like(self.search.query.to_owned()),
                SearchMode::Variable => reading.like(format!("%{}%", self.search.query)),
                SearchMode::LeftVariable => reading.like(format!("%{}", self.search.query)),
                SearchMode::RightVariable => reading.like(format!("{}%", self.search.query)),
            }
        };

        // Wait for tokio-diesel to support boxed queries #20
        if self.search.limit > 0 {
            Ok(dict
                .select(sequence)
                .filter(predicate)
                .limit(self.search.limit as i64)
                .get_results_async(&self.db)
                .await?)
        } else {
            Ok(dict
                .select(sequence)
                .filter(predicate)
                .get_results_async(&self.db)
                .await?)
        }
    }

    /// Load all senses for the sequence ids
    async fn load_senses(&self, sequence_ids: &Vec<i32>) -> Result<Vec<sense::Sense>, Error> {
        use crate::schema::sense as sense_schema;

        // Always search by a language.
        let lang = self.language.unwrap_or(Language::default());

        let senses: Vec<sense::Sense> = sense_schema::table
            .filter(
                sense_schema::sequence.eq_any(sequence_ids).and(
                    sense_schema::language
                        .eq(lang)
                        .or(sense_schema::language.eq(Language::default())),
                ),
            )
            .get_results_async(&self.db)
            .await?;

        Ok(senses)
    }

    /// Load readings for all sequences
    async fn load_readings(&self, sequence_ids: &Vec<i32>) -> Result<Vec<Item>, Error> {
        use crate::schema::dict as dict_schema;

        // load dicts from DB
        let dicts: Vec<Dict> = dict_schema::table
            .filter(dict_schema::sequence.eq_any(sequence_ids))
            .order_by(dict_schema::id)
            .get_results_async(&self.db)
            .await?;

        Ok(dicts
            .into_iter()
            .group_by(|i| i.sequence)
            .into_iter()
            .map(|(seq, dicts)| {
                let mut reading = Reading {
                    sequence: seq,
                    ..Default::default()
                };
                let mut priorities: Option<Vec<Priority>> = None;
                let mut information: Option<Vec<Information>> = None;

                dicts.for_each(|dict| {
                    if priorities.is_none() && dict.priorities.is_some() {
                        priorities = dict.priorities.clone();
                    }
                    if information.is_none() && dict.information.is_some() {
                        information = dict.information.clone();
                    }

                    if reading.kana.is_none() && !dict.kanji {
                        reading.kana = Some(dict);
                        return;
                    }

                    if reading.kanji.is_none() && dict.kanji {
                        reading.kanji = Some(dict);
                        return;
                    }

                    reading.alternative.push(dict);
                });

                Item {
                    reading,
                    priorities,
                    information,
                    sequence: seq,
                    ..Default::default()
                }
            })
            .collect_vec())
    }
}

/*
/// Search for words based on the provided query
pub async fn search_word(db: &DbPool, query: &str) -> Result<Vec<Item>, Error> {
    let mut result: Vec<Item> = Vec::new();

    if has_kanji(query) {
        // Search only for japanese words
        result.extend(search_readings(db, query).await?);
    } else {
        // Search for non-jp words
        result.extend(search_glosses(query)?);

        // Allow explicit searches with query = "term"
        if !query.starts_with('"') || !query.ends_with('"') {
            // search in hiragana
            result.extend(search_glosses(query.to_hiragana().as_str())?);

            // search in katakana
            result.extend(search_glosses(query.to_katakana().as_str())?);
        }
    }

    Ok(vec![])
}

/// Searchs for translated 'meanings'
fn search_glosses(query: &str) -> Result<Vec<Item>, Error> {
    Ok(vec![])
}
*/

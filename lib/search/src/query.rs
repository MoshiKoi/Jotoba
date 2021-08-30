use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    str::FromStr,
};

use super::query_parser::QueryType;

use itertools::Itertools;
use resources::{
    models::kanji,
    parse::jmdict::{languages::Language, part_of_speech::PosSimple},
};

/// A single user provided query in a parsed format
#[derive(Debug, Clone, PartialEq, Default, Hash)]
pub struct Query {
    pub original_query: String,
    pub query: String,
    pub type_: QueryType,
    pub tags: Vec<Tag>,
    pub form: Form,
    pub language: QueryLang,
    pub settings: UserSettings,
    pub page: usize,
    pub word_index: usize,
    pub parse_japanese: bool,
}

/// In-cookie saved personalized settings
#[derive(Debug, Clone, Copy)]
pub struct UserSettings {
    pub user_lang: Language,
    pub page_lang: localization::language::Language,
    pub show_english: bool,
    pub english_on_top: bool,
    pub cookies_enabled: bool,
}

impl PartialEq for UserSettings {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.user_lang == other.user_lang && self.show_english == other.show_english
    }
}

impl Hash for UserSettings {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.user_lang.hash(state);
        self.show_english.hash(state);
    }
}

impl Default for UserSettings {
    #[inline]
    fn default() -> Self {
        Self {
            show_english: true,
            user_lang: Language::default(),
            page_lang: localization::language::Language::default(),
            english_on_top: false,
            cookies_enabled: false,
        }
    }
}

/// Hashtag based search tags
#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub enum Tag {
    SearchType(SearchTypeTag),
    PartOfSpeech(PosSimple),
}

/// Hashtag based search tags
#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub enum SearchTypeTag {
    Kanji,
    Sentence,
    Name,
    Word,
}

/// The language of the query
#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub enum QueryLang {
    Japanese,
    Foreign,
    Undetected,
}

/// The form the query was provided in
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Form {
    /// A single word was provided
    SingleWord,
    /// Multiple words were provided
    MultiWords,
    /// Kanji reading based search eg. '気 ケ'
    KanjiReading(kanji::Reading),
    /// Form was not recognized
    Undetected,
}

impl Form {
    #[inline]
    pub fn as_kanji_reading(&self) -> Option<&kanji::Reading> {
        if let Self::KanjiReading(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the form is [`KanjiReading`].
    pub fn is_kanji_reading(&self) -> bool {
        matches!(self, Self::KanjiReading(..))
    }
}

impl Default for Form {
    #[inline]
    fn default() -> Self {
        Self::Undetected
    }
}

impl Default for QueryLang {
    #[inline]
    fn default() -> Self {
        Self::Undetected
    }
}

impl Tag {
    // Parse a tag from a string
    pub fn parse_from_str(s: &str) -> Option<Tag> {
        Some(if let Some(tag) = Self::parse_search_type(s) {
            tag
        } else {
            match PosSimple::from_str(&s[1..]) {
                Ok(pos) => Self::PartOfSpeech(pos),
                Err(_) => return None,
            }
        })
    }

    /// Parse only search type
    pub fn parse_search_type(s: &str) -> Option<Tag> {
        Some(match s[1..].to_lowercase().as_str() {
            "kanji" => Self::SearchType(SearchTypeTag::Kanji),
            "sentence" | "sentences" => Self::SearchType(SearchTypeTag::Sentence),
            "name" | "names" => Self::SearchType(SearchTypeTag::Name),
            "word" | "words" => Self::SearchType(SearchTypeTag::Word),
            _ => return None,
        })
    }

    /// Returns `true` if the tag is [`SearchType`].
    #[inline]
    pub fn is_search_type(&self) -> bool {
        matches!(self, Self::SearchType(..))
    }

    /// Returns `true` if the tag is [`PartOfSpeech`].
    #[inline]
    pub fn is_part_of_speech(&self) -> bool {
        matches!(self, Self::PartOfSpeech(..))
    }

    #[inline]
    pub fn as_search_type(&self) -> Option<&SearchTypeTag> {
        if let Self::SearchType(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[inline]
    pub fn as_part_of_speech(&self) -> Option<&PosSimple> {
        if let Self::PartOfSpeech(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl Query {
    #[inline]
    pub fn is_valid(&self) -> bool {
        !self.query.is_empty()
    }

    #[inline]
    pub fn get_hash(&self) -> u64 {
        let mut hash = DefaultHasher::new();
        self.hash(&mut hash);
        hash.finish()
    }

    /// Returns true if the query has at least one pos tag
    #[inline]
    pub fn has_part_of_speech_tags(&self) -> bool {
        !self.get_part_of_speech_tags().is_empty()
    }

    /// Returns all search type tags
    #[inline]
    pub fn get_search_type_tags(&self) -> Vec<SearchTypeTag> {
        self.tags
            .iter()
            .filter(|i| i.is_search_type())
            .map(|i| i.as_search_type().unwrap())
            .copied()
            .collect()
    }

    /// Returns all PosSimple tags
    #[inline]
    pub fn get_part_of_speech_tags(&self) -> Vec<PosSimple> {
        self.tags
            .iter()
            .filter(|i| i.is_part_of_speech())
            .map(|i| i.as_part_of_speech().unwrap())
            .copied()
            .collect()
    }

    /// Returns the original_query with search type tags omitted
    #[inline]
    pub fn without_search_type_tags(&self) -> String {
        self.original_query
            .clone()
            .split(' ')
            .into_iter()
            .filter(|i| {
                // Filter out all search type tags
                (i.starts_with('#') && Tag::parse_search_type(i).is_none()) || !i.starts_with('#')
            })
            .join(" ")
    }
}

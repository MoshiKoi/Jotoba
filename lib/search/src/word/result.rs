use std::path::Path;

use super::super::query::Query;

use japanese::{
    accent::{AccentChar, Border},
    furigana::{self, SentencePartRef},
    inflection::Inflection,
    JapaneseExt,
};
use jp_inflections::{Verb, VerbType, WordForm};
use localization::{language::Language as locLanguage, traits::Translatable, TranslationDict};
use parse::jmdict::{
    dialect::Dialect,
    field::Field,
    gtype::GType,
    information::Information,
    languages::Language,
    misc::Misc,
    part_of_speech::{PartOfSpeech, PosSimple},
    priority::Priority,
};

use itertools::Itertools;
use utils::to_option;

use models::{dict::Dict, kanji::KanjiResult, sense::Sense as DbSenseEntry};

#[derive(Debug, Clone, PartialEq)]
pub struct WordResult {
    pub items: Vec<Item>,
    pub count: usize,
    pub contains_kanji: bool,
    pub inflection_info: Option<InflectionInformation>,
}

impl WordResult {
    pub fn has_word(&self) -> bool {
        self.items.iter().any(|i| i.is_word())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InflectionInformation {
    pub lexeme: String,
    pub forms: Vec<Inflection>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Word(Word),
    Kanji(KanjiResult),
}

impl Item {
    /// Returns `true` if the item is [`Word`].
    pub fn is_word(&self) -> bool {
        matches!(self, Self::Word(..))
    }

    /// Returns `true` if the item is [`Kanji`].
    pub fn is_kanji(&self) -> bool {
        matches!(self, Self::Kanji(..))
    }
}

impl From<KanjiResult> for Item {
    fn from(k: KanjiResult) -> Self {
        Self::Kanji(k)
    }
}

impl From<Word> for Item {
    fn from(w: Word) -> Self {
        Self::Word(w)
    }
}

/// A single word item
#[derive(Debug, Clone, Default)]
pub struct Word {
    pub sequence: i32,
    pub priorities: Option<Vec<Priority>>,
    pub information: Option<Vec<Information>>,
    pub reading: Reading,
    pub senses: Vec<Sense>,
    pub collocations: Option<Vec<(String, String)>>,
}

impl PartialEq for Word {
    fn eq(&self, other: &Self) -> bool {
        // At this state, the sequence is unique for each element
        self.sequence == other.sequence
    }
}

/// Various readins of a word
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Reading {
    pub sequence: i32,
    pub kana: Option<Dict>,
    pub kanji: Option<Dict>,
    pub alternative: Vec<Dict>,
}

/// A single sense for a word. Represents one language,
/// one misc item and 1..n glosses
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sense {
    pub language: Language,
    pub misc: Option<Misc>,
    pub field: Option<Field>,
    pub dialect: Option<Dialect>,
    pub glosses: Vec<Gloss>,
    pub xref: Option<String>,
    pub antonym: Option<String>,
    pub information: Option<String>,
}

/// A gloss value represents one word in the
/// translated language.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Gloss {
    pub gloss: String,
    pub g_type: Option<GType>,
    pub part_of_speech: Vec<PartOfSpeech>,
    pub pos_simple: Vec<PosSimple>,
}

impl From<Vec<DbSenseEntry>> for Sense {
    fn from(entry: Vec<DbSenseEntry>) -> Self {
        let first = &entry[0];
        let gtype = &first.gtype;
        Sense {
            language: first.language,
            misc: first.misc,
            field: first.field,
            dialect: first.dialect,
            xref: first.xref.clone(),
            antonym: first.antonym.clone(),
            information: first.information.clone(),
            glosses: entry
                .clone()
                .into_iter()
                .map(|i| Gloss {
                    part_of_speech: i.part_of_speech.unwrap_or_default(),
                    pos_simple: i.pos_simplified.unwrap_or_default(),
                    g_type: (*gtype),
                    gloss: i.gloss,
                })
                .collect_vec(),
        }
    }
}

//
// Small handy functions used in the templates //
//

impl Word {
    /// Get alternative readings in a beautified, print-ready format
    pub fn alt_readings_beautified(&self) -> String {
        self.reading
            .alternative
            .iter()
            .map(|i| i.reading.clone())
            .join(", ")
    }

    /// Returns true if a word is common
    pub fn is_common(&self) -> bool {
        self.reading.get_reading().priorities.is_some()
    }

    /// Returns the reading of a word
    pub fn get_reading(&self) -> &Dict {
        self.reading.get_reading()
    }

    pub fn glosses_pretty(&self) -> String {
        let senses = self.get_senses();

        // Try to use glosses with users language
        if !senses[0].is_empty() {
            Self::pretty_print_senses(&senses[0])
        } else {
            // Fallback use english gloses
            Self::pretty_print_senses(&senses[1])
        }
    }

    fn pretty_print_senses(senses: &[Sense]) -> String {
        senses
            .iter()
            .map(|i| i.glosses.clone())
            .flatten()
            .into_iter()
            .map(|i| i.gloss)
            .join(", ")
    }

    /// Returns furigana reading-pairs of an Item
    pub fn get_furigana(&self) -> Option<Vec<SentencePartRef<'_>>> {
        let furi = self.get_reading().furigana.as_ref()?;
        Some(furigana::from_str(furi).collect_vec())
        /*
        if self.reading.kanji.is_some() && self.reading.kana.is_some() {
            furigana::pairs_checked(
                self.reading
                    .kanji
                    .as_ref()
                    .map(|i| i.reading.as_str())
                    .unwrap(),
                self.reading
                    .kana
                    .as_ref()
                    .map(|i| i.reading.as_str())
                    .unwrap(),
            )
        } else {
            None
        }
        */
    }

    /// Return true if item has a certain reading
    pub fn has_reading(&self, reading: &str, ignore_case: bool) -> bool {
        if let Some(kanji) = self.reading.kanji.as_ref().map(|i| &i.reading) {
            if (ignore_case && kanji.to_lowercase() == reading.to_lowercase()) || (kanji == reading)
            {
                return true;
            }
        }

        if let Some(kana) = self.reading.kana.as_ref().map(|i| &i.reading) {
            if (ignore_case && kana.to_lowercase() == reading.to_lowercase()) || (kana == reading) {
                return true;
            }
        }

        false
    }

    /// Get senses ordered by language (non-english first)
    pub fn get_senses(&self) -> Vec<Vec<Sense>> {
        let (english, other): (Vec<Sense>, Vec<Sense>) = self
            .senses
            .clone()
            .into_iter()
            .partition(|i| i.language == Language::English);

        vec![other, english]
    }

    /// Get senses ordered by language (non-english first)
    pub fn get_senses_orderd(&self, query: &Query) -> Vec<Vec<Sense>> {
        let (english, other): (Vec<Sense>, Vec<Sense>) = self
            .senses
            .clone()
            .into_iter()
            .partition(|i| i.language == Language::English);

        if query.settings.english_on_top {
            vec![english, other]
        } else {
            vec![other, english]
        }
    }

    /// Return all senses of a language
    pub fn senses_by_lang(&self, language: Language) -> Option<Vec<Sense>> {
        to_option(
            self.senses
                .iter()
                .filter(|i| i.language == language)
                .cloned()
                .collect_vec(),
        )
    }

    /// Get amount of tags which will be displayed below the reading
    pub fn get_word_tag_count(&self) -> u8 {
        [self.is_common(), self.get_reading().jlpt_lvl.is_some()]
            .iter()
            .filter(|b| **b)
            .count() as u8
    }

    /// Return true if item is a katakana word
    pub fn is_katakana_word(&self) -> bool {
        self.reading.is_katakana()
    }

    /// Get the audio filename of a word
    pub fn audio_file(&self) -> Option<String> {
        self.reading.kanji.as_ref().and_then(|kanji| {
            let file = format!(
                "{}【{}】.ogg",
                kanji.reading,
                self.reading.kana.as_ref().unwrap().reading
            );

            Path::new(&format!("html/assets/audio/{}", file))
                .exists()
                .then(|| file)
        })
    }

    /// Returns a renderable vec of accents with kana characters
    pub fn get_accents(&self) -> Option<Vec<AccentChar>> {
        let kana = self.reading.kana.as_ref().unwrap();
        let accents = kana.get_accents()?;
        let accent_iter = accents.iter().peekable().enumerate();

        Some(
            accent_iter
                .map(|(pos, (part, is_high))| {
                    let borders = vec![if *is_high {
                        Border::Top
                    } else {
                        Border::Bottom
                    }];
                    let borders = if pos != accents.len() - 1 {
                        borders.into_iter().chain(vec![Border::Right]).collect_vec()
                    } else {
                        borders
                    };
                    vec![AccentChar { borders, c: part }]
                })
                .flatten()
                .into_iter()
                .collect_vec(),
        )
    }

    fn get_pos(&self) -> impl Iterator<Item = &PartOfSpeech> {
        self.senses
            .iter()
            .map(|i| i.glosses[0].part_of_speech.iter())
            .flatten()
    }

    /// Returns a jp_inflections::Verb if [`self`] is a verb
    fn get_jp_verb(&self) -> Option<Verb> {
        let verb_type = if self.get_pos().any(|i| i.is_ichidan()) {
            VerbType::Ichidan
        } else if self.get_pos().any(|i| i.is_godan()) {
            VerbType::Godan
        } else {
            return None;
        };

        let verb = Verb::new(
            jp_inflections::Word::new(
                self.reading.kana.as_ref().map(|i| &i.reading).unwrap(),
                self.reading.kanji.as_ref().map(|i| &i.reading),
            ),
            verb_type,
        );

        // Check if [`verb`] really is a valid verb in dictionary form
        verb.word.is_verb().then(|| verb)
    }

    /// Returns an [`Inflections`] value if [`self`] is a valid verb
    pub fn get_inflections(&self) -> Option<Inflections> {
        let verb = self.get_jp_verb()?;

        let build = || -> Result<Inflections, jp_inflections::error::Error> {
            Ok(Inflections {
                present: InflectionPair {
                    positive: verb.dictionary(WordForm::Short)?.get_reading(),
                    negative: verb.negative(WordForm::Short)?.get_reading(),
                },
                present_polite: InflectionPair {
                    positive: verb.dictionary(WordForm::Long)?.get_reading(),
                    negative: verb.negative(WordForm::Long)?.get_reading(),
                },

                past: InflectionPair {
                    positive: verb.past(WordForm::Short)?.get_reading(),
                    negative: verb.negative_past(WordForm::Short)?.get_reading(),
                },
                past_polite: InflectionPair {
                    positive: verb.past(WordForm::Long)?.get_reading(),
                    negative: verb.negative_past(WordForm::Long)?.get_reading(),
                },
                te_form: InflectionPair {
                    positive: verb.te_form()?.get_reading(),
                    negative: verb.negative_te_form()?.get_reading(),
                },
                potential: InflectionPair {
                    positive: verb.potential(WordForm::Short)?.get_reading(),
                    negative: verb.negative_potential(WordForm::Short)?.get_reading(),
                },
                passive: InflectionPair {
                    positive: verb.passive()?.get_reading(),
                    negative: verb.negative_passive()?.get_reading(),
                },
                causative: InflectionPair {
                    positive: verb.causative()?.get_reading(),
                    negative: verb.negative_causative()?.get_reading(),
                },
            })
        }()
        .ok()?;

        Some(build)
    }
}

pub struct Inflections {
    pub present: InflectionPair,
    pub present_polite: InflectionPair,

    pub past: InflectionPair,
    pub past_polite: InflectionPair,

    pub te_form: InflectionPair,

    pub potential: InflectionPair,
    pub passive: InflectionPair,
    pub causative: InflectionPair,
}

pub struct InflectionPair {
    pub positive: String,
    pub negative: String,
}

impl Reading {
    /// Return true if reading represents a katakana only word
    pub fn is_katakana(&self) -> bool {
        self.kana.as_ref().unwrap().reading.is_katakana() && self.kanji.is_none()
    }

    /// Returns the word-reading of a Reading object
    pub fn get_reading(&self) -> &Dict {
        self.kanji
            .as_ref()
            .unwrap_or_else(|| self.kana.as_ref().unwrap())
    }

    /// Returns the jplt level of a word. None if
    /// a word doesn't have a JPLT lvl assigned
    pub fn get_jplt_lvl(&self) -> Option<i32> {
        self.get_reading().jlpt_lvl
    }
}

impl Sense {
    // Get a senses tags prettified
    pub fn get_glosses(&self) -> String {
        self.glosses.iter().map(|i| i.gloss.clone()).join("; ")
    }

    pub fn get_xref(&self) -> Option<&str> {
        self.xref.as_ref().and_then(|xref| xref.split('・').next())
    }

    pub fn get_antonym(&self) -> Option<&str> {
        self.antonym
            .as_ref()
            .and_then(|antonym| antonym.split('・').next())
    }

    pub fn get_infos(
        &self,
        dict: &TranslationDict,
        language: locLanguage,
    ) -> Option<(Option<String>, Option<&str>, Option<&str>, Option<Dialect>)> {
        let info_str = self.get_information_string(dict, language);
        let xref = self.get_xref();
        let antonym = self.get_antonym();
        let dialect = self.dialect;

        if xref.is_none() && info_str.is_none() && antonym.is_none() {
            None
        } else {
            Some((info_str, xref, antonym, dialect))
        }
    }

    /// Return human readable information about a gloss
    pub fn get_information_string(
        &self,
        dict: &TranslationDict,
        language: locLanguage,
    ) -> Option<String> {
        let arr: [Option<String>; 3] = [
            self.misc
                .map(|i| i.gettext(dict, Some(language)).to_owned()),
            self.field.map(|i| i.gettext_custom(dict, Some(language))),
            self.information.clone(),
        ];

        let res = arr
            .iter()
            .filter_map(|i| i.is_some().then(|| i.as_ref().unwrap()))
            .collect_vec();

        if res.is_empty() {
            return None;
        }

        if self.xref.is_some() || self.antonym.is_some() {
            Some(format!("{}.", res.iter().join(", ")))
        } else {
            Some(res.iter().join(", "))
        }
    }

    // Get a senses tags prettified
    pub fn get_parts_of_speech(&self, dict: &TranslationDict, language: locLanguage) -> String {
        self.glosses[0]
            .part_of_speech
            .iter()
            .map(|i| i.gettext_custom(dict, Some(language)))
            .join(", ")
    }

    // Get all pos_simple
    pub fn get_pos_simple(&self) -> Vec<PosSimple> {
        self.glosses
            .iter()
            .map(|i| i.pos_simple.to_owned())
            .flatten()
            .collect::<Vec<_>>()
    }
}

fn map_to_str<T>(i: &Option<T>) -> Option<String>
where
    T: Into<String> + Copy,
{
    i.as_ref().map(|i| {
        let s: String = (*i).into();
        s
    })
}

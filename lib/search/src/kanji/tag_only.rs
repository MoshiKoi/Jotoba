use error::Error;

use crate::query::{Query, Tag};

use super::KanjiResult;

pub fn search(query: &Query) -> Result<KanjiResult, Error> {
    let single_tag = query.tags.iter().find(|i| i.is_empty_allowed());

    if single_tag.is_none() {
        return Ok(KanjiResult::default());
    }

    match single_tag.unwrap() {
        Tag::Jlpt(jlpt) => jlpt_search(query, *jlpt),
        Tag::GenkiLesson(genki_lesson) => genki_search(query, *genki_lesson),
        _ => return Ok(KanjiResult::default()),
    }
}

fn genki_search(query: &Query, genki_lesson: u8) -> Result<KanjiResult, Error> {
    let kanji_retrieve = resources::get().kanji();

    let genki_lesson = kanji_retrieve.by_genki_lesson(genki_lesson);

    if genki_lesson.is_none() {
        return Ok(KanjiResult::default());
    }

    let kanji = genki_lesson
        // we ensured that there is a genki lesson above
        .unwrap()
        .iter()
        .filter_map(|literal| kanji_retrieve.by_literal(*literal))
        .cloned()
        .collect::<Vec<_>>();

    let len = kanji.len();

    let page_offset = query.page_offset(query.settings.kanji_page_size as usize);

    let kanji = kanji
        .into_iter()
        .skip(page_offset)
        .take(query.settings.kanji_page_size as usize)
        .collect::<Vec<_>>();

    let items = super::to_item(kanji, query);

    Ok(KanjiResult {
        items,
        total_items: len,
    })
}

fn jlpt_search(query: &Query, jlpt: u8) -> Result<KanjiResult, Error> {
    let kanji_retrieve = resources::get().kanji();

    let jlpt_kanji = match kanji_retrieve.by_jlpt(jlpt) {
        Some(jlpt) => jlpt,
        None => return Ok(KanjiResult::default()),
    };

    let len = jlpt_kanji.len();

    let page_offset = query.page_offset(query.settings.kanji_page_size as usize);

    let jlpt_kanji = jlpt_kanji
        .into_iter()
        .skip(page_offset)
        .take(query.settings.kanji_page_size as usize)
        .filter_map(|literal| kanji_retrieve.by_literal(*literal))
        .cloned()
        .collect::<Vec<_>>();

    Ok(KanjiResult {
        items: super::to_item(jlpt_kanji, query),
        total_items: len,
    })
}

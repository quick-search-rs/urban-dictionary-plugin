use abi_stable::{
    export_root_module,
    prefix_type::PrefixTypeTrait,
    sabi_extern_fn,
    sabi_trait::prelude::TD_Opaque,
    std_types::{RBox, RStr, RString, RVec},
};
use quick_search_lib::{ColoredChar, Log, PluginId, SearchLib, SearchLib_Ref, SearchResult, Searchable, Searchable_TO};

static NAME: &str = "Urban Dictionary";

#[export_root_module]
pub fn get_library() -> SearchLib_Ref {
    SearchLib { get_searchable }.leak_into_prefix()
}

#[sabi_extern_fn]
fn get_searchable(id: PluginId, logger: quick_search_lib::ScopedLogger) -> Searchable_TO<'static, RBox<()>> {
    let this = UrbanDictionary::new(id, logger);
    Searchable_TO::from_value(this, TD_Opaque)
}

struct UrbanDictionary {
    id: PluginId,
    client: reqwest::blocking::Client,
    logger: quick_search_lib::ScopedLogger,
}

impl UrbanDictionary {
    fn new(id: PluginId, logger: quick_search_lib::ScopedLogger) -> Self {
        Self {
            id,
            client: reqwest::blocking::Client::new(),
            logger,
        }
    }
}

impl Searchable for UrbanDictionary {
    fn search(&self, query: RString) -> RVec<SearchResult> {
        let mut res: Vec<(SearchResult, i32)> = vec![];

        match DictionaryApiResponse::get_word(&query, &self.client) {
            Ok(response) => {
                for meaning in response.list {
                    res.push((
                        SearchResult::new(&meaning.definition)
                            .set_context(&meaning.example)
                            .set_extra_info(&format!("{}\n{}", meaning.definition, meaning.example)),
                        meaning.thumbs_up - meaning.thumbs_down,
                    ))
                }
            }
            Err(e) => {
                self.logger.error(&format!("failed to get word: {}", e));
            }
        }

        res.sort_by(|a, b| a.0.title().cmp(b.0.title()));
        res.dedup_by(|a, b| a.0.title() == b.0.title());
        res.sort_by(|a, b| a.1.cmp(&b.1));

        res.into_iter().map(|x| x.0).collect()
    }
    fn name(&self) -> RStr<'static> {
        NAME.into()
    }
    fn colored_name(&self) -> RVec<quick_search_lib::ColoredChar> {
        // can be dynamic although it's iffy how it might be used
        // ColoredChar::from_string(NAME, 0x16BE2FFF)
        vec![
            ColoredChar::new_rgba('U', 228, 247, 20, 255),
            ColoredChar::new_rgba('r', 228, 247, 20, 255),
            ColoredChar::new_rgba('b', 228, 247, 20, 255),
            ColoredChar::new_rgba('a', 228, 247, 20, 255),
            ColoredChar::new_rgba('n', 228, 247, 20, 255),
            ColoredChar::new_rgba(' ', 228, 247, 20, 255),
            ColoredChar::new_rgba('D', 17, 78, 232, 255),
            ColoredChar::new_rgba('i', 17, 78, 232, 255),
            ColoredChar::new_rgba('c', 17, 78, 232, 255),
            ColoredChar::new_rgba('t', 17, 78, 232, 255),
            ColoredChar::new_rgba('i', 17, 78, 232, 255),
            ColoredChar::new_rgba('o', 17, 78, 232, 255),
            ColoredChar::new_rgba('n', 17, 78, 232, 255),
            ColoredChar::new_rgba('a', 17, 78, 232, 255),
            ColoredChar::new_rgba('r', 17, 78, 232, 255),
            ColoredChar::new_rgba('y', 17, 78, 232, 255),
        ]
        .into()
    }
    fn execute(&self, result: &SearchResult) {
        let s = result.extra_info();
        if let Ok::<clipboard::ClipboardContext, Box<dyn std::error::Error>>(mut clipboard) = clipboard::ClipboardProvider::new() {
            if let Ok(()) = clipboard::ClipboardProvider::set_contents(&mut clipboard, s.to_owned()) {
                self.logger.trace(&format!("copied to clipboard: {}", s));
            } else {
                self.logger.error(&format!("failed to copy to clipboard: {}", s));
            }
        } else {
            self.logger.error(&format!("failed to copy to clipboard: {}", s));
        }

        // finish up, above is a clipboard example
    }
    fn plugin_id(&self) -> PluginId {
        self.id.clone()
    }
}

#[derive(serde::Deserialize, Debug)]
struct DictionaryApiResponse {
    // word: String,
    // phonetic: String,
    // phonetics: Vec<Phonetic>,
    list: Vec<Meaning>,
    // license: License,
    // #[serde(rename = "sourceUrls")]
    // source_urls: Vec<String>,
}

// #[derive(serde::Deserialize, Debug)]
// struct Phonetic {
//     text: String,
//     audio: String,
//     #[serde(rename = "sourceUrl")]
//     source_url: Option<String>,
//     license: Option<License>,
// }

#[derive(serde::Deserialize, Debug, Clone)]
struct Meaning {
    definition: String,
    // permalink: String,
    thumbs_up: i32,
    // author: String,
    // word: String,
    // defid: i32,
    // current_vote: String,
    // written_on: String,
    example: String,
    thumbs_down: i32,
}

impl DictionaryApiResponse {
    fn get_word(word: &str, client: &reqwest::blocking::Client) -> anyhow::Result<Self> {
        let url = format!("https://api.urbandictionary.com/v0/define?term={}", urlencoding::encode(word));

        let response = client.get(url).send()?;

        let mut json: Self = response.json()?;

        json.list.iter_mut().for_each(|meaning| {
            meaning.definition = meaning.definition.replace(['[', ']', '\r', '\n'], "");
            meaning.example = meaning.example.replace(['[', ']', '\r', '\n'], "");
        });

        Ok(json)
    }
}

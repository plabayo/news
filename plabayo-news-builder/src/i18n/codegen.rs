use std::fs::{self, File};
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use convert_case::{Case, Casing};
use itertools::Itertools;

use crate::i18n::locales::{Storage, StringValuePathPair};

pub fn generate_locales(dir: &str, storage: &Storage) -> Result<()> {
    fs::create_dir_all(dir)?;
    let file_path = Path::new(dir).join("locales.rs");
    let file = File::create(file_path)
        .with_context(|| format!("create locales rust file at {}/locales.rs", dir))?;

    let default_locales = storage
        .get_default()
        .ok_or_else(|| anyhow!("failed to get default locale in i18n storage"))?;

    generate_locales_enum(&file, storage)
    .with_context(|| format!("generate locales enum definition and its methods/traits implementation in {}/locales.rs", dir))?;

    let default_pairs: Vec<StringValuePathPair> = default_locales.iter().collect();

    generate_locales_strings_struct(
        &file,
        default_pairs.iter().map(|p| p.path.clone()).collect(),
    )
    .with_context(|| {
        format!(
            "generate locales strings struct (def) in {}/locales.rs",
            dir
        )
    })?;

    let default_pairs_stringified: Vec<StringValuePathPair> = default_pairs
        .iter()
        .map(|p| StringValuePathPair {
            value: format!(
                r#################"r################"{}"################"#################,
                p.value
            ),
            path: p.path.clone(),
        })
        .collect();
    generate_locales_strings_instance(&file, "STRINGS_DEFAULT", default_pairs_stringified.iter())
        .with_context(|| {
        format!(
            "generate strings for default locale {} in {}/locales.rs",
            storage.default_locale(),
            dir
        )
    })?;

    for locale in storage
        .all_locales()
        .filter(|locale| locale != &storage.default_locale())
    {
        let iter = LocaleStringWithDefaultIter::new(
            storage
                .get(locale)
                .ok_or_else(|| anyhow!("failed to get strings for locale {}", locale))?
                .iter(),
            default_pairs.clone().into_iter(),
        );
        let pairs: Vec<StringValuePathPair> = iter.collect();
        generate_locales_strings_instance(
            &file,
            &format!("STRINGS_{}", locale.to_case(Case::ScreamingSnake)),
            pairs.iter(),
        )
        .with_context(|| {
            format!(
                "generate strings for locale {} in {}/locales.rs",
                locale, dir
            )
        })?;
    }

    Ok(())
}

fn generate_locales_enum(mut w: impl std::io::Write, storage: &Storage) -> Result<()> {
    // 1. generate enum type

    w.write_all(
        b"pub enum Locales {
",
    )?;
    for locale in storage.all_locales() {
        w.write_all(
            format!(
                "    {},
",
                locale.to_case(Case::Pascal)
            )
            .as_bytes(),
        )?;
    }
    w.write_all(
        b"}

",
    )?;

    let default_locale = storage.default_locale();

    // 2. impl methods on our enum type (to get strings for a locale)

    w.write_all(
        b"impl Locales {
    pub fn strings(&self) -> &Strings {
        match self {
",
    )?;
    for locale in storage.all_locales() {
        w.write_all(
            format!(
                r#"            Self::{} => &STRINGS_{},
"#,
                locale.to_case(Case::Pascal),
                if locale == default_locale {
                    "DEFAULT".to_owned()
                } else {
                    locale.to_case(Case::ScreamingSnake)
                }
            )
            .as_bytes(),
        )?;
    }
    w.write_all(
        b"        }
    }

",
    )?;

    w.write_all(
        b"    pub fn as_str(&self) -> &str {
        match self {",
    )?;
    for locale in storage.all_locales() {
        w.write_all(
            format!(
                r#"
            Self::{} => "{}","#,
                locale.to_case(Case::Pascal),
                locale.to_case(Case::Kebab),
            )
            .as_bytes(),
        )?;
    }
    w.write_all(
        b"
        }
    }
}

",
    )?;

    // 3. impl conversation from str, for our enum type

    w.write_all(
        b"impl From<&str> for Locales {
    fn from(s: &str) -> Self {
        match s.to_lowercase().trim() {
",
    )?;
    for locale in storage.all_locales() {
        w.write_all(
            format!(
                r#"            "{}" => Self::{},
"#,
                locale.to_lowercase().trim(),
                locale.to_case(Case::Pascal)
            )
            .as_bytes(),
        )?;
    }
    w.write_all(
        b"            _ => DEFAULT_LOCALE,
",
    )?;
    w.write_all(
        b"        }
    }
}

",
    )?;

    // 4. generate default locale constant

    w.write_all(
        format!(
            "pub const DEFAULT_LOCALE: Locales = Locales::{};

",
            default_locale.to_case(Case::Pascal)
        )
        .as_bytes(),
    )?;

    // x. all good

    Ok(())
}

fn generate_locales_strings_struct(
    mut w: impl std::io::Write,
    mut paths: Vec<Vec<String>>,
) -> Result<()> {
    let mut layer: usize = 0;
    while !paths.is_empty() {
        if layer == 0 {
            w.write_all(
                b"pub struct Strings {
",
            )?;
        }
        let mut previous: Option<String> = None;
        let mut previous_property: Option<String> = None;
        let mut retained_paths = Vec::new();
        for path in paths {
            // create new struct if needed
            let current = if layer == 0 {
                None
            } else {
                Some(path[layer - 1].clone())
            };
            if previous != current {
                w.write_all(
                    b"}

",
                )?;
                w.write_all(
                    format!(
                        "pub struct Strings{} {{
",
                        path[..layer]
                            .iter()
                            .map(|s| s.to_case(Case::Pascal))
                            .join("")
                    )
                    .as_bytes(),
                )?;
                previous = current;
            }

            let key = &path[layer];
            let current_property = Some(key.clone());
            let drop = path.len() == layer + 1;

            // write struct property
            if drop {
                // str
                w.write_all(
                    format!(
                        "    pub {}: &'static str,
",
                        key.to_case(Case::Snake)
                    )
                    .as_bytes(),
                )?;
            } else if current_property != previous_property {
                // object
                w.write_all(
                    format!(
                        "    pub {}: Strings{},
",
                        key.to_case(Case::Snake),
                        path[..layer + 1]
                            .iter()
                            .map(|s| s.to_case(Case::Pascal))
                            .join("")
                    )
                    .as_bytes(),
                )?;
                previous_property = current_property;
            }

            // retain if we do not wish to drop
            if !drop {
                retained_paths.push(path);
            }
        }

        layer += 1;
        paths = retained_paths;
    }
    w.write_all(
        b"}
",
    )?;
    Ok(())
}

fn generate_locales_strings_instance<'a>(
    mut w: impl std::io::Write,
    const_name: &str,
    pairs: impl Iterator<Item = &'a StringValuePathPair>,
) -> Result<()> {
    w.write_all(
        format!(
            "
const {}: Strings = Strings {{
",
            const_name
        )
        .as_bytes(),
    )?;
    let mut previous_layer = 0;
    let mut previous_path = None;
    // for each locale string...
    for pair in pairs {
        let current_layer = pair.path.len() - 1;
        match current_layer.cmp(&previous_layer) {
            std::cmp::Ordering::Greater => {
                // handle case in case we are indenting more (creating a child)
                while current_layer > previous_layer {
                    let key = &pair.path[previous_layer];
                    w.write_all(
                        format!(
                            "{}{}: Strings{} {{
",
                            "    ".repeat(previous_layer + 1),
                            key.to_case(Case::Snake),
                            pair.path[..=previous_layer]
                                .iter()
                                .map(|s| s.to_case(Case::Pascal))
                                .join(""),
                        )
                        .as_bytes(),
                    )?;
                    previous_layer += 1;
                }
            }
            std::cmp::Ordering::Less => {
                // as well as the case where we indenting less (ending a child)
                while current_layer < previous_layer {
                    previous_layer -= 1;
                    w.write_all(
                        format!(
                            "{}}},
",
                            "    ".repeat(previous_layer + 1)
                        )
                        .as_bytes(),
                    )?;
                }
            }
            std::cmp::Ordering::Equal => {
                // and finally handle the cases where we go from one nested child to another
                let mut overlap_layer = 0;
                if let Some(previous_path) = previous_path {
                    for (key_a, key_b) in pair.path.iter().zip(previous_path) {
                        if key_a != key_b {
                            break;
                        }
                        overlap_layer += 1;
                    }
                    if overlap_layer < previous_layer {
                        for idx in 0..(previous_layer - overlap_layer) {
                            w.write_all(
                                format!(
                                    "{}}},
",
                                    "    ".repeat(previous_layer - idx)
                                )
                                .as_bytes(),
                            )?;
                        }
                        while overlap_layer < previous_layer {
                            let key = &pair.path[overlap_layer];
                            w.write_all(
                                format!(
                                    "{}{}: Strings{} {{
",
                                    "    ".repeat(overlap_layer + 1),
                                    key.to_case(Case::Snake),
                                    pair.path[..=overlap_layer]
                                        .iter()
                                        .map(|s| s.to_case(Case::Pascal))
                                        .join(""),
                                )
                                .as_bytes(),
                            )?;
                            overlap_layer += 1;
                        }
                    }
                }
            }
        }
        // write the actual locale string...
        let key = &pair.path[previous_layer];
        w.write_all(
            format!(
                r#################"{}{}: {},
"#################,
                "    ".repeat(previous_layer + 1),
                key.to_case(Case::Snake),
                pair.value
            )
            .as_bytes(),
        )?;
        // keep track of the previous path to be handle the more complex nesting cases
        previous_path = Some(&pair.path);
    }
    // add all the final curly brackets... including the last one
    while previous_layer > 0 {
        previous_layer -= 1;
        w.write_all(
            format!(
                "{}}},
",
                "    ".repeat(previous_layer + 1)
            )
            .as_bytes(),
        )?;
    }
    w.write_all(
        b"};
",
    )?;
    Ok(())
}

struct LocaleStringWithDefaultIter<
    T: Iterator<Item = StringValuePathPair>,
    U: Iterator<Item = StringValuePathPair>,
> {
    pairs: Box<T>,
    default_pairs: Box<U>,
    next_pair: Option<StringValuePathPair>,
    next_default_pair: Option<StringValuePathPair>,
}

impl<T: Iterator<Item = StringValuePathPair>, U: Iterator<Item = StringValuePathPair>>
    LocaleStringWithDefaultIter<T, U>
{
    pub fn new(pairs: T, mut default_pairs: U) -> LocaleStringWithDefaultIter<T, U> {
        let next_default_pair = default_pairs.next();
        LocaleStringWithDefaultIter {
            pairs: Box::new(pairs),
            default_pairs: Box::new(default_pairs),
            next_pair: None,
            next_default_pair,
        }
    }
}

impl<T: Iterator<Item = StringValuePathPair>, U: Iterator<Item = StringValuePathPair>> Iterator
    for LocaleStringWithDefaultIter<T, U>
{
    type Item = StringValuePathPair;

    fn next(&mut self) -> Option<Self::Item> {
        // load the next item to render
        match std::mem::replace(&mut self.next_default_pair, None) {
            // if there is no next default pair,
            // than we can immediately stop as it means we're finished,
            // with all possible properties,
            // anything left in our main pairs iter are non-standard properties
            None => None,
            Some(next_default_pair) => {
                loop {
                    // get the last peeked pair if there was one,
                    // or else get the next one, so we can start comparing
                    let pair = if self.next_pair.is_some() {
                        std::mem::replace(&mut self.next_pair, None)
                    } else {
                        self.pairs.next()
                    };
                    // if we didn't found a pair we'll need to start filling up all defaults
                    let pair = match pair {
                        Some(pair) => pair,
                        None => {
                            // missing keys, we'll fill up...
                            self.next_default_pair = self.default_pairs.next();
                            return Some(StringValuePathPair {
                                path: next_default_pair.path.clone(),
                                value: format!(
                                    "STRINGS_DEFAULT.{}",
                                    next_default_pair
                                        .path
                                        .iter()
                                        .map(|s| s.to_case(Case::Snake))
                                        .join("."),
                                ),
                            });
                        }
                    };
                    // if we have a match we mean our pairs has the required property at the current
                    // position and thus we render the correct value
                    if pair == next_default_pair {
                        self.next_default_pair = self.default_pairs.next();
                        return Some(StringValuePathPair {
                            path: pair.path.clone(),
                            value: format!(
                                r#################"r################"{}"################"#################,
                                pair.value
                            ),
                        });
                    }
                    // in case we have not yet reached the current next default pair,
                    // we want to skip the current pair, as it is a non-standard one
                    if pair < next_default_pair {
                        continue;
                    }
                    // our next pair is already beyond the next desired property path,
                    // so we need to fill up until we reach our pair's current path
                    self.next_default_pair = self.default_pairs.next();
                    // keep our fetched pair for next time
                    self.next_pair = Some(pair);
                    return Some(StringValuePathPair {
                        path: next_default_pair.path.clone(),
                        value: format!(
                            "STRINGS_DEFAULT.{}",
                            next_default_pair
                                .path
                                .iter()
                                .map(|s| s.to_case(Case::Snake))
                                .join("."),
                        ),
                    });
                }
            }
        }
    }
}

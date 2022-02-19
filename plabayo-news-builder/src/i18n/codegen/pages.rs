// Plabayo News
// Copyright (C) 2021  Glen Henri J. De Cauwsemaecker
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::fs::{self, File};
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use convert_case::{Case, Casing};

use crate::i18n::codegen::common::generate_copyright_file_header;
use crate::i18n::config::StaticPages;
use crate::i18n::locales::Storage;

pub fn generate_pages(file_path: &Path, storage: &Storage, cfg: &StaticPages) -> Result<()> {
    println!("cargo:rerun-if-changed={}", cfg.path);

    let file = File::create(file_path)
        .with_context(|| format!("create locales rust file at {}", file_path.display()))?;

    let (not_found_template, templates) = get_templates(&cfg.path, &cfg.not_found)
        .with_context(|| format!("get templates for result at {}", file_path.display()))?;

    generate_copyright_file_header(&file).with_context(|| {
        format!(
            "generate locales module copyright (header) in {}",
            file_path.display()
        )
    })?;

    generate_pages_mod_docs(&file).with_context(|| {
        format!(
            "generate pages module docs (header) in {}",
            file_path.display()
        )
    })?;

    generate_pages_imports(&file)
        .with_context(|| format!("generate pages imports in {}", file_path.display()))?;

    generate_pages_static_response(&file, &templates[..], not_found_template.as_str())
        .with_context(|| {
            format!(
                "generate pages static response pub core API method in {}",
                file_path.display()
            )
        })?;

    generate_pages_local_utility_functions(&file).with_context(|| {
        format!(
            "generate pages local utility functions in {}",
            file_path.display()
        )
    })?;

    generate_pages_is_static_root(&file, &templates[..], not_found_template.as_str())
        .with_context(|| {
            format!(
                "generate pages 'is_static_root' pub utility {}",
                file_path.display()
            )
        })?;

    generate_pages_static_pages(&file, storage, &templates[..], not_found_template.as_str())
        .with_context(|| {
            format!(
                "generate pages static page functionality in {}",
                file_path.display()
            )
        })?;

    generate_pages_templates_mod(&file, &templates[..], cfg.templates_dir.as_str()).with_context(
        || {
            format!(
                "generate pages static page functionality in {}",
                file_path.display()
            )
        },
    )?;

    Ok(())
}

fn get_templates(templates_path: &str, not_found: &str) -> Result<(String, Vec<String>)> {
    let paths = fs::read_dir(templates_path)
        .with_context(|| format!("list all static page templates in {}", templates_path))?;
    let not_found_template = not_found.to_owned();
    let mut templates = vec![not_found_template.clone()];
    for path in paths {
        let path = path
            .with_context(|| format!("list a static page template found in {}", templates_path))?
            .path();
        let name = path
            .file_stem()
            .ok_or_else(|| {
                anyhow!(
                    "get file stem of a static page template found in {}",
                    templates_path
                )
            })?
            .to_str()
            .ok_or_else(|| {
                anyhow!(
                    "convert file stem of static page template found in {} to &str",
                    templates_path
                )
            })?;
        if name != not_found {
            templates.push(name.to_owned());
        }
    }
    Ok((not_found_template, templates))
}

fn generate_pages_mod_docs(mut w: impl std::io::Write) -> Result<()> {
    w.write_all(
        b"//! this pages module is auto-generated by the plabayo-news-builder::i18n crate.
//! DO NOT MODIFY MANUALLY AS IT WILL BE OVERWRITTEN NEXT TIME YOU BUILD USING CARGO!!!
//! ... Best to also not check in this file into remote repo.
",
    )?;
    Ok(())
}

fn generate_pages_imports(mut w: impl std::io::Write) -> Result<()> {
    w.write_all(
        b"use actix_web::{http::StatusCode, HttpResponse};
use lazy_static::lazy_static;

use crate::site::assets;
use crate::site::l18n::locales::Locale;
use crate::site::SITE_INFO;

",
    )?;
    Ok(())
}

fn generate_pages_static_response(
    mut w: impl std::io::Write,
    templates: &[String],
    not_found: &str,
) -> Result<()> {
    w.write_all(
        b"pub async fn static_response(locale: Locale, endpoint: &str) -> HttpResponse {
    match endpoint {
",
    )?;
    for template in templates {
        if template == not_found {
            continue;
        }
        w.write_all(
            format!(
                "        PAGE_{}_ENDPOINT => static_page_{}(locale),
",
                template.to_case(Case::ScreamingSnake),
                template.to_case(Case::Snake)
            )
            .as_bytes(),
        )?;
    }
    w.write_all(
        format!(
            "        _ => static_page_{}(locale),
",
            not_found.to_case(Case::Snake)
        )
        .as_bytes(),
    )?;

    w.write_all(
        b"    }
}

",
    )?;

    Ok(())
}

fn generate_pages_local_utility_functions(mut w: impl std::io::Write) -> Result<()> {
    w.write_all(
        b"fn static_page(status_code: StatusCode, body: &'static str) -> HttpResponse {
    HttpResponse::build(status_code)
        .content_type(\"text/html\")
        .body(body)
}

",
    )?;
    Ok(())
}

fn generate_pages_static_pages(
    mut w: impl std::io::Write,
    storage: &Storage,
    templates: &[String],
    not_found: &str,
) -> Result<()> {
    for template in templates {
        w.write_all(
            format!(
                "fn static_page_{}(locale: Locale) -> HttpResponse {{
    static_page(
        StatusCode::{},
        match locale {{
",
                template.to_case(Case::Snake),
                if template == not_found {
                    "NOT_FOUND"
                } else {
                    "OK"
                }
            )
            .as_bytes(),
        )?;
        for locale in storage.all_locales() {
            w.write_all(
                format!(
                    "            Locale::{} => PAGE_{}_{}.as_str(),
",
                    locale.to_case(Case::Pascal),
                    template.to_case(Case::ScreamingSnake),
                    locale.to_case(Case::ScreamingSnake)
                )
                .as_bytes(),
            )?;
        }

        w.write_all(
            b"        },
    )
}

",
        )?;

        if template != not_found {
            w.write_all(
                format!(
                    r##"const PAGE_{}_ENDPOINT: &str = "{}";
"##,
                    template.to_case(Case::ScreamingSnake),
                    template.to_case(Case::Kebab),
                )
                .as_bytes(),
            )?;
        }

        w.write_all(
            format!(
                r##"const PAGE_{}_PATH: &str = "/{}";

lazy_static! {{
"##,
                template.to_case(Case::ScreamingSnake),
                template.to_case(Case::Kebab)
            )
            .as_bytes(),
        )?;

        for locale in storage.all_locales() {
            w.write_all(
                format!(
                    r##"    static ref PAGE_{}_{}: String =
        templates::{}::response_body(Locale::{}, PAGE_{}_PATH, &SITE_INFO);
"##,
                    template.to_case(Case::ScreamingSnake),
                    locale.to_case(Case::ScreamingSnake),
                    template.to_case(Case::Pascal),
                    locale.to_case(Case::Pascal),
                    template.to_case(Case::ScreamingSnake)
                )
                .as_bytes(),
            )?;
        }

        w.write_all(
            b"}

",
        )?;
    }

    Ok(())
}

fn generate_pages_is_static_root(
    mut w: impl std::io::Write,
    templates: &[String],
    not_found: &str,
) -> Result<()> {
    w.write_all(
        b"pub fn is_static_root(root: &str) -> bool {
    matches!(
        root.to_lowercase().as_str(),
        assets::ROOT
",
    )?;
    for template in templates {
        if template.as_str() == not_found {
            continue;
        }
        w.write_all(
            format!(
                "            | PAGE_{}_ENDPOINT
",
                template.to_case(Case::ScreamingSnake)
            )
            .as_bytes(),
        )?
    }
    w.write_all(
        b"    )
}

",
    )?;

    Ok(())
}

fn generate_pages_templates_mod(
    mut w: impl std::io::Write,
    templates: &[String],
    templates_dir: &str,
) -> Result<()> {
    w.write_all(
        b"mod templates {
    use askama::Template;

    use super::*;

    use crate::site::templates::PageState;
    use crate::site::SiteInfo;",
    )?;

    for template in templates {
        w.write_all(
            format!(
                r##"

    #[derive(Template)]
    #[template(path = "{}/{}.html", escape = "none")]
    pub struct {}<'a> {{
        site_info: &'a SiteInfo,
        page: PageState<'a>,
    }}

    impl<'a> {}<'a> {{
        pub fn response_body(locale: Locale, path: &'a str, info: &'a SiteInfo) -> String {{
            {} {{
                site_info: info,
                // TODO: make userInfo not required for static pages at this point?!
                page: PageState::new(locale, path, None, None),
            }}
            .render()
            .unwrap()
        }}
    }}"##,
                templates_dir,
                template,
                template.to_case(Case::Pascal),
                template.to_case(Case::Pascal),
                template.to_case(Case::Pascal)
            )
            .as_bytes(),
        )?;
    }

    w.write_all(
        b"
}
",
    )?;

    Ok(())
}

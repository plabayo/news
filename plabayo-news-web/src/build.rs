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

use actix_web_static_files::resource_dir;
use anyhow::Result;
use vergen::{vergen, Config};

use plabayo_news_builder::i18n;

fn main() -> Result<()> {
    // Generate the default 'cargo:' instruction output
    vergen(Config::default())?;

    // build the i18n locale structs and (Askama) templates
    // for the website's static pages.
    i18n::build("./Cargo.toml")?;

    // Bundle static resources so we can serve these from memory,
    // and make the setup of the news web server easier.
    resource_dir("./site/assets").build()?;

    // All good.
    Ok(())
}

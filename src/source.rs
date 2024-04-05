use chrono::{Local, NaiveDate};
use regex::Regex;
use sqlx::FromRow;
use tracing::*;

use crate::config::{Config, FormatStandard};

#[derive(Debug, FromRow, Clone)]
pub struct Source {
    pub id: i64,
    pub title: String,
    pub url: String,
    pub author: String,
    pub published_date: NaiveDate,
    pub viewed_date: NaiveDate,
    pub published_date_unknown: bool,
    pub comment: String,
}

impl Source {
    pub fn format(&self, standard: &FormatStandard) -> String {
        trace!("Formatting source with: {:?}", standard);

        match standard {
            FormatStandard::Default => {
                let mut out = String::new();

                out.push_str(format!("[{}]", self.id).as_str());

                match self.author.is_empty() {
                    true => out.push_str(" Unbekannt"),
                    false => out.push_str(format!(" {}", self.author).as_str()),
                }

                if !self.published_date_unknown {
                    out.push_str(format!(" ({})", self.published_date.format("%Y")).as_str());
                }

                out.push_str(
                    format!(
                        ": {} URL: {} [Stand: {}]",
                        self.title,
                        self.url,
                        self.viewed_date.format("%d. %m. %Y")
                    )
                    .as_str(),
                );

                out
            }
            FormatStandard::Custom => {
                let config = Config::get_config();

                // get custom date format from string
                let viewed_date_format = {
                    let regex = Regex::new(r"\{V_DATE\((?<format>[^)]*)\)}").unwrap();
                    match regex.captures(&config.custom_format) {
                        None => "%d. %m. %Y".to_string(),
                        Some(cap) => {
                            if cap["format"].to_string().is_empty() {
                                "%d. %m. %Y".to_string()
                            } else {
                                cap["format"].to_string()
                            }
                        }
                    }
                };

                // get custom date format from string
                let published_date_format = {
                    let regex = Regex::new(r"\{P_DATE\((?<format>[^)]*)\)}").unwrap();
                    match regex.captures(&config.custom_format) {
                        None => "%d. %m. %Y".to_string(),
                        Some(cap) => {
                            if cap["format"].to_string().is_empty() {
                                "%d. %m. %Y".to_string()
                            } else {
                                cap["format"].to_string()
                            }
                        }
                    }
                };

                let mut out = config.custom_format;

                let mut replace = |regex: &str, text: &str| {
                    let regex = Regex::new(regex).expect("Faulty regex");
                    out = regex.replace_all(&out, text).to_string();
                };

                replace(r"\{INDEX\}", &self.id.to_string());
                replace(r"\{TITLE\}", &self.title);
                replace(r"\{URL\}", &self.url);
                replace(r"\{AUTHOR\}", &self.author);

                // replace {P_DATE(*)} with the custom date
                if self.published_date_unknown {
                    replace(r"\{P_DATE\([^)]*\)}", "Unknown");
                } else {
                    replace(
                        r"\{P_DATE\([^)]*\)}",
                        &self
                            .published_date
                            .format(&published_date_format)
                            .to_string(),
                    );
                }

                // replace {V_DATE(*)} with the custom date
                replace(
                    r"\{V_DATE\([^)]*\)}",
                    &self.viewed_date.format(&viewed_date_format).to_string(),
                );

                out
            }
        }
    }

    pub fn contains(&self, query: &str) -> bool {
        if self.title.to_lowercase().contains(&query.to_lowercase())
            || self.url.to_lowercase().contains(&query.to_lowercase())
            || self.author.to_lowercase().contains(&query.to_lowercase())
        {
            return true;
        }

        false
    }
}

impl Default for Source {
    fn default() -> Self {
        trace!("Creating new Source");

        Self {
            id: -1,
            title: String::new(),
            author: String::new(),
            url: String::new(),
            published_date: Local::now().date_naive(), // current date
            viewed_date: Local::now().date_naive(),    // current date
            published_date_unknown: false,
            comment: String::new(),
        }
    }
}

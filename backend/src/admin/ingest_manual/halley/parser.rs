//! Pure HTML parsing for the Halley "delibere" listing/detail pages
//! (`halleyweb.com`). No HTTP, no `kb-store`. Tested against real, captured
//! markup (see `halley/fixtures/`), not invented structure — Halley's markup
//! is a third-party CMS, outside this project's control.

use chrono::NaiveDate;
use scraper::{Html, Selector};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct HalleyListingRow {
    pub act_type: String,
    pub number: String,
    pub date: NaiveDate,
    pub title: String,
    pub detail_path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HalleyDetailDocument {
    pub oggetto: String,
    pub attachment_path: String,
    pub attachment_filename: String,
}

#[derive(Debug, PartialEq)]
pub enum HalleyParseError {
    MissingListingTable,
    MalformedRow(String),
    MissingOggetto,
    MissingAttachment,
}

impl fmt::Display for HalleyParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HalleyParseError::MissingListingTable => {
                write!(
                    f,
                    "Halley listing page markup not recognized (missing #table-delibere-public)"
                )
            }
            HalleyParseError::MalformedRow(msg) => write!(f, "malformed Halley listing row: {msg}"),
            HalleyParseError::MissingOggetto => {
                write!(f, "Halley detail page missing an 'Oggetto' field")
            }
            HalleyParseError::MissingAttachment => {
                write!(
                    f,
                    "Halley detail page missing a 'Documento' attachment link"
                )
            }
        }
    }
}

impl std::error::Error for HalleyParseError {}

pub fn parse_listing(html: &str) -> Result<Vec<HalleyListingRow>, HalleyParseError> {
    let document = Html::parse_document(html);
    let table_selector = Selector::parse("#table-delibere-public").unwrap();
    if document.select(&table_selector).next().is_none() {
        return Err(HalleyParseError::MissingListingTable);
    }

    let row_selector = Selector::parse("#table-delibere-public tr[data-href]").unwrap();
    let cell_selector = Selector::parse("td.hidden-xs").unwrap();
    let link_selector = Selector::parse("a").unwrap();

    let mut result = Vec::new();
    for row in document.select(&row_selector) {
        let detail_path = row
            .value()
            .attr("data-href")
            .ok_or_else(|| HalleyParseError::MalformedRow("row has no data-href".into()))?
            .to_string();

        let cells: Vec<_> = row.select(&cell_selector).collect();
        if cells.len() < 4 {
            return Err(HalleyParseError::MalformedRow(format!(
                "expected 4 td.hidden-xs cells, found {}",
                cells.len()
            )));
        }

        let act_type = cells[0].text().collect::<String>().trim().to_string();
        let number = cells[1].text().collect::<String>().trim().to_string();
        let date_str = cells[2].text().collect::<String>().trim().to_string();
        let date = NaiveDate::parse_from_str(&date_str, "%d/%m/%Y").map_err(|_| {
            HalleyParseError::MalformedRow(format!("unparseable date '{date_str}'"))
        })?;
        let title = cells[3]
            .select(&link_selector)
            .next()
            .map(|a| a.text().collect::<String>().trim().to_string())
            .ok_or_else(|| HalleyParseError::MalformedRow("title cell has no <a>".into()))?;

        result.push(HalleyListingRow {
            act_type,
            number,
            date,
            title,
            detail_path,
        });
    }

    Ok(result)
}

pub fn parse_detail(html: &str) -> Result<HalleyDetailDocument, HalleyParseError> {
    let document = Html::parse_document(html);
    let row_selector = Selector::parse(".detail-row").unwrap();
    let label_selector = Selector::parse(".detail-label").unwrap();
    let value_selector = Selector::parse(".detail-value").unwrap();
    let link_selector = Selector::parse("a[href]").unwrap();

    let mut oggetto: Option<String> = None;
    let mut attachment: Option<(String, String)> = None;

    for row in document.select(&row_selector) {
        let label = match row.select(&label_selector).next() {
            Some(l) => l.text().collect::<String>().trim().to_string(),
            None => continue,
        };
        let value_el = match row.select(&value_selector).next() {
            Some(v) => v,
            None => continue,
        };

        match label.as_str() {
            "Oggetto" => {
                oggetto = Some(value_el.text().collect::<String>().trim().to_string());
            }
            "Documento" => {
                if let Some(a) = value_el.select(&link_selector).next() {
                    let href = a.value().attr("href").unwrap_or_default().to_string();
                    let filename = a.text().collect::<String>().trim().to_string();
                    attachment = Some((href, filename));
                }
            }
            _ => {}
        }
    }

    let oggetto = oggetto.ok_or(HalleyParseError::MissingOggetto)?;
    let (attachment_path, attachment_filename) =
        attachment.ok_or(HalleyParseError::MissingAttachment)?;

    Ok(HalleyDetailDocument {
        oggetto,
        attachment_path,
        attachment_filename,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTING_FIXTURE: &str = include_str!("fixtures/delibere-listing-page1.html");
    const DETAIL_FIXTURE: &str = include_str!("fixtures/delibera-detail-example.html");

    #[test]
    fn should_parse_five_rows_from_the_real_listing_page() {
        let rows = parse_listing(LISTING_FIXTURE).expect("parse failed");
        assert_eq!(rows.len(), 5);
    }

    #[test]
    fn should_parse_the_first_row_with_exact_real_values() {
        let rows = parse_listing(LISTING_FIXTURE).expect("parse failed");
        let first = &rows[0];
        assert_eq!(first.act_type, "Delibera Di Giunta");
        assert_eq!(first.number, "74");
        assert_eq!(first.date, NaiveDate::from_ymd_opt(2026, 7, 13).unwrap());
        assert!(first.title.contains("POSTEGGI AREA FIERA SANT'ANNA"));
        assert_eq!(
            first.detail_path,
            "/c042023/zf/index.php/atti-amministrativi/delibere/dettaglio/atto/GTlRFekE9RT0-H"
        );
    }

    #[test]
    fn should_parse_rows_in_newest_first_order() {
        let rows = parse_listing(LISTING_FIXTURE).expect("parse failed");
        for pair in rows.windows(2) {
            assert!(
                pair[0].date >= pair[1].date,
                "expected newest-first ordering, got {:?} before {:?}",
                pair[0].date,
                pair[1].date
            );
        }
    }

    #[test]
    fn should_return_missing_listing_table_error_for_unrelated_html() {
        let result = parse_listing("<html><body>not a delibere page</body></html>");
        assert_eq!(result, Err(HalleyParseError::MissingListingTable));
    }

    #[test]
    fn should_parse_the_real_detail_page() {
        let detail = parse_detail(DETAIL_FIXTURE).expect("parse failed");
        assert!(detail.oggetto.contains("POSTEGGI AREA FIERA SANT'ANNA"));
        assert_eq!(
            detail.attachment_path,
            "/c042023/zf/../de/attachment.php?serialDocumento=00ZOBA020267L"
        );
        assert!(
            detail
                .attachment_filename
                .contains("delibera copia uso amministrativo.pdf")
        );
    }

    #[test]
    fn should_return_missing_oggetto_error_for_unrelated_html() {
        let result = parse_detail("<html><body>not a detail page</body></html>");
        assert_eq!(result, Err(HalleyParseError::MissingOggetto));
    }
}

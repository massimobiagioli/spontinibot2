//! Non-interactive curation for `halleyweb.com`'s "delibere" listing (Plan
//! 0030) — an explicit, config-driven exception to `ingest-core`'s
//! unconditional robots.txt enforcement, scoped to one named, operator-
//! authorized domain. See the plan's Risks section and the accompanying ADR.

use std::sync::Arc;
use std::time::Duration;

use kb_store::KbStore;
use url::Url;

use crate::admin::ingest_manual::{
    IngestManualAdminPort, IngestManualError, IngestManualResponse, RecencyWindow,
};
use crate::admin::upload::extractors::CompositeExtractor;
use crate::admin::upload::ports::UploadPort;
use crate::admin::upload::preview_store::UploadMetadata;
use crate::admin::upload::tagging::extract_tags;

use super::parser::{HalleyListingRow, parse_detail, parse_listing};

const MAX_TAGS: usize = 5;
const TRUST_SCORE: f32 = 0.9;
const POLITENESS_DELAY: Duration = Duration::from_millis(300);
const MAX_PAGES: u32 = 1000;

pub struct HalleyCurationAdapter {
    store: Arc<KbStore>,
    upload: Arc<dyn UploadPort>,
    client: reqwest::Client,
}

impl HalleyCurationAdapter {
    pub fn new(store: Arc<KbStore>, upload: Arc<dyn UploadPort>) -> Self {
        Self {
            store,
            upload,
            client: reqwest::Client::builder()
                .user_agent("spontini-halley-curation/0.1")
                .build()
                .expect("failed to build HTTP client"),
        }
    }

    fn listing_page_url(src: &str, page: u32) -> String {
        if page == 1 {
            src.to_string()
        } else {
            format!("{src}/index/table-delibere-public-page/{page}")
        }
    }

    fn resolve(src: &str, path: &str) -> Result<String, IngestManualError> {
        let base = Url::parse(src)
            .map_err(|e| IngestManualError::Ingest(format!("invalid source URL: {e}")))?;
        base.join(path)
            .map(|u| u.to_string())
            .map_err(|e| IngestManualError::Ingest(format!("failed to resolve '{path}': {e}")))
    }

    async fn fetch_text(&self, url: &str) -> Result<String, IngestManualError> {
        tokio::time::sleep(POLITENESS_DELAY).await;
        self.client
            .get(url)
            .send()
            .await
            .map_err(|e| IngestManualError::Ingest(format!("fetch failed for {url}: {e}")))?
            .text()
            .await
            .map_err(|e| IngestManualError::Ingest(format!("failed to read body of {url}: {e}")))
    }

    async fn fetch_bytes(&self, url: &str) -> Result<Vec<u8>, IngestManualError> {
        tokio::time::sleep(POLITENESS_DELAY).await;
        let bytes = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| IngestManualError::Ingest(format!("fetch failed for {url}: {e}")))?
            .bytes()
            .await
            .map_err(|e| IngestManualError::Ingest(format!("failed to read body of {url}: {e}")))?;
        Ok(bytes.to_vec())
    }

    /// Collects every listing row within `[cutoff, bookmark)`, newest-first,
    /// across as many pages as needed (or `MAX_PAGES`, whichever comes
    /// first — a safety cap against unexpected markup drift on this
    /// third-party CMS).
    async fn collect_rows_within_window(
        &self,
        src: &str,
        cutoff: chrono::NaiveDate,
        bookmark_ref: Option<&str>,
    ) -> Result<Vec<HalleyListingRow>, IngestManualError> {
        let mut collected = Vec::new();

        for page in 1..=MAX_PAGES {
            let page_url = Self::listing_page_url(src, page);
            let html = self.fetch_text(&page_url).await?;
            let rows = parse_listing(&html)
                .map_err(|e| IngestManualError::Ingest(format!("Halley listing: {e}")))?;
            if rows.is_empty() {
                break;
            }

            let mut reached_boundary = false;
            for row in rows {
                if row.date < cutoff {
                    reached_boundary = true;
                    break;
                }
                if bookmark_ref.is_some_and(|r| r == row.number) {
                    reached_boundary = true;
                    break;
                }
                collected.push(row);
            }
            if reached_boundary {
                break;
            }
        }

        Ok(collected)
    }

    /// Halley reuses the same generic attachment filename (e.g. "delibera
    /// copia uso amministrativo.pdf") across many unrelated acts — using it
    /// verbatim as `source_ref` would collapse distinct documents into one
    /// indistinguishable, uncitable blob (confirmed live: 178 chunks from
    /// dozens of different delibere all merged under that one label before
    /// this fix). Build a filename unique to the act itself instead,
    /// preserving only the real extension from the original filename.
    fn unique_filename(row: &HalleyListingRow, original_filename: &str) -> String {
        let ext = original_filename.rsplit('.').next().unwrap_or("bin");
        let type_slug: String = row
            .act_type
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect();
        format!("{type_slug}-{}-{}.{ext}", row.number, row.date)
    }

    async fn curate_one(
        &self,
        src: &str,
        section: &str,
        row: &HalleyListingRow,
    ) -> Result<(), IngestManualError> {
        let detail_url = Self::resolve(src, &row.detail_path)?;
        let detail_html = self.fetch_text(&detail_url).await?;
        let detail = parse_detail(&detail_html)
            .map_err(|e| IngestManualError::Ingest(format!("Halley detail: {e}")))?;

        let attachment_url = Self::resolve(src, &detail.attachment_path)?;
        let pdf_bytes = self.fetch_bytes(&attachment_url).await?;

        let filename = Self::unique_filename(row, &detail.attachment_filename);

        let extracted = CompositeExtractor::extract(&pdf_bytes, &filename)
            .map_err(|e| IngestManualError::Ingest(e.to_string()))?;

        let derived_tags = extract_tags(&extracted.content, MAX_TAGS);
        let metadata = UploadMetadata {
            category: Some(section.to_string()),
            tags: if derived_tags.is_empty() {
                None
            } else {
                Some(derived_tags)
            },
            trust_score: Some(TRUST_SCORE),
            summary: Some(detail.oggetto.clone()),
            source_url: Some(detail_url.clone()),
        };

        self.upload
            .ingest_uploaded(&extracted.content, section, &filename, &metadata)
            .await
            .map_err(|e| IngestManualError::Ingest(e.to_string()))?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl IngestManualAdminPort for HalleyCurationAdapter {
    async fn ingest(
        &self,
        section: &str,
        src: &str,
        window: RecencyWindow,
    ) -> Result<IngestManualResponse, IngestManualError> {
        let sections = self
            .store
            .list_sections()
            .await
            .map_err(|e| IngestManualError::Ingest(e.to_string()))?;
        let section_id = sections
            .iter()
            .find(|s| s.name == section)
            .map(|s| s.id)
            .ok_or_else(|| IngestManualError::Ingest(format!("section '{section}' not found")))?;

        let bookmark = self
            .store
            .get_bookmark(section_id, src)
            .await
            .map_err(|e| IngestManualError::Ingest(e.to_string()))?;

        let cutoff = window.cutoff_date(chrono::Utc::now().date_naive());
        let bookmark_ref = bookmark.as_ref().map(|b| b.last_item_ref.as_str());
        let rows = self
            .collect_rows_within_window(src, cutoff, bookmark_ref)
            .await?;

        if rows.is_empty() {
            return Ok(IngestManualResponse {
                section: section.to_string(),
                src: src.to_string(),
                window: window.to_string(),
                status: "no new items".to_string(),
            });
        }

        // The bookmark is written immediately after each item succeeds, not
        // batched to the end of the loop: a long real run against a live
        // third-party site can be cut short at any point (client timeout,
        // network blip, operator Ctrl-C), which drops this future wherever
        // it currently is and runs no further code at all. Checkpointing
        // per-item bounds the "redo on retry" risk to at most the single
        // item in flight, instead of the whole remaining batch.
        for row in rows.iter().rev() {
            self.curate_one(src, section, row).await?;
            self.store
                .upsert_bookmark(section_id, src, &row.number, &row.date.to_string())
                .await
                .map_err(|e| IngestManualError::Ingest(e.to_string()))?;
        }

        Ok(IngestManualResponse {
            section: section.to_string(),
            src: src.to_string(),
            window: window.to_string(),
            status: format!("ingested {} document(s)", rows.len()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Mutex;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// (text, section, filename, summary, source_url) recorded per
    /// `ingest_uploaded` call.
    type RecordedCall = (String, String, String, Option<String>, Option<String>);

    struct RecordingUploadPort {
        calls: Mutex<Vec<RecordedCall>>,
        fail_on_filename: Option<String>,
    }

    impl RecordingUploadPort {
        fn new() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                fail_on_filename: None,
            }
        }

        fn failing_on(filename: &str) -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                fail_on_filename: Some(filename.to_string()),
            }
        }
    }

    #[async_trait]
    impl UploadPort for RecordingUploadPort {
        async fn ingest_uploaded(
            &self,
            text: &str,
            section: &str,
            filename: &str,
            metadata: &UploadMetadata,
        ) -> Result<Vec<i64>, crate::admin::upload::UploadError> {
            if self.fail_on_filename.as_deref() == Some(filename) {
                return Err(crate::admin::upload::UploadError::IngestFailed(
                    "simulated failure".into(),
                ));
            }
            self.calls.lock().unwrap().push((
                text.to_string(),
                section.to_string(),
                filename.to_string(),
                metadata.summary.clone(),
                metadata.source_url.clone(),
            ));
            Ok(vec![1])
        }
    }

    fn listing_html(rows: &[(&str, &str)]) -> String {
        let rows_html: String = rows
            .iter()
            .map(|(number, date)| {
                format!(
                    r#"<tr data-href='/detail/{number}'>
                        <td class='hidden-xs nospace'>Delibera Di Giunta</td>
                        <td class='text-right hidden-xs'>{number}</td>
                        <td class='hidden-xs'>{date}</td>
                        <td class='hidden-xs'><div class='truncate-ellipsis'><span><a href='/detail/{number}'>Atto numero {number}</a></span></div></td>
                    </tr>"#
                )
            })
            .collect();
        format!(
            r#"<html><body><div id="table-delibere-public"><table><tbody>{rows_html}</tbody></table></div></body></html>"#
        )
    }

    fn detail_html(number: &str, attachment_url: &str) -> String {
        detail_html_with_filename(number, attachment_url, &format!("atto-{number}.txt"))
    }

    fn detail_html_with_filename(number: &str, attachment_url: &str, filename: &str) -> String {
        format!(
            r#"<html><body>
                <div class="row detail-row"><div class="detail-label">Oggetto</div><div class="detail-value">Atto numero {number}</div></div>
                <div class="row detail-row"><div class="detail-label">Documento</div><div class="detail-value"><a href="{attachment_url}">{filename}</a></div></div>
            </body></html>"#
        )
    }

    static DB_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

    async fn temp_store() -> Arc<KbStore> {
        let n = DB_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("halley_curation_test_{n}.db"));
        let _ = std::fs::remove_file(&path);
        Arc::new(
            KbStore::open(&path.to_string_lossy())
                .await
                .expect("failed to open temp db"),
        )
    }

    #[tokio::test]
    async fn should_ingest_every_row_within_window_across_two_pages_on_first_run() {
        let server = MockServer::start().await;
        let src = server.uri();

        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(listing_html(&[("75", "20/07/2026"), ("74", "13/07/2026")])),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/index/table-delibere-public-page/2"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(listing_html(&[("73", "01/01/2026")])),
            )
            .mount(&server)
            .await;

        for number in ["75", "74"] {
            Mock::given(method("GET"))
                .and(path(format!("/detail/{number}")))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_string(detail_html(number, &format!("/attachment/{number}.txt"))),
                )
                .mount(&server)
                .await;
            Mock::given(method("GET"))
                .and(path(format!("/attachment/{number}.txt")))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_string(format!("Contenuto dell'atto numero {number}.")),
                )
                .mount(&server)
                .await;
        }

        let store = temp_store().await;
        store
            .upsert_section(kb_store::NewIngestSection {
                name: "delibere".into(),
                ordering: 0,
            })
            .await
            .expect("section insert failed");

        let upload = Arc::new(RecordingUploadPort::new());
        let adapter = HalleyCurationAdapter::new(store, upload.clone());

        let response = adapter
            .ingest("delibere", &src, RecencyWindow::Days(30))
            .await
            .expect("curation failed");

        assert_eq!(response.status, "ingested 2 document(s)");
        let calls = upload.calls.lock().unwrap();
        assert_eq!(calls.len(), 2);
        assert!(
            calls
                .iter()
                .any(|(_, _, f, _, _)| f == "delibera-di-giunta-75-2026-07-20.txt")
        );
        assert!(
            calls
                .iter()
                .any(|(_, _, f, _, _)| f == "delibera-di-giunta-74-2026-07-13.txt")
        );
    }

    #[tokio::test]
    async fn should_persist_the_halley_oggetto_field_as_the_document_summary() {
        // Regression test: `parse_detail` already extracts each act's real
        // official "Oggetto" (subject) field, but it used to be discarded
        // after parsing instead of being persisted anywhere — leaving the
        // ingested-document detail card with no way to show what a curated
        // document is actually about.
        let server = MockServer::start().await;
        let src = server.uri();

        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(listing_html(&[("74", "13/07/2026")])),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/index/table-delibere-public-page/2"))
            .respond_with(ResponseTemplate::new(200).set_body_string(listing_html(&[])))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/detail/74"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(detail_html("74", "/attachment/74.txt")),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/attachment/74.txt"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Contenuto dell'atto 74."))
            .mount(&server)
            .await;

        let store = temp_store().await;
        store
            .upsert_section(kb_store::NewIngestSection {
                name: "delibere".into(),
                ordering: 0,
            })
            .await
            .expect("section insert failed");

        let upload = Arc::new(RecordingUploadPort::new());
        let adapter = HalleyCurationAdapter::new(store, upload.clone());

        adapter
            .ingest("delibere", &src, RecencyWindow::Days(30))
            .await
            .expect("curation failed");

        let calls = upload.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].3.as_deref(),
            Some("Atto numero 74"),
            "the uploaded metadata's summary must match the real Oggetto text \
             from the detail page, not be discarded"
        );
        assert_eq!(
            calls[0].4.as_deref(),
            Some(format!("{src}/detail/74").as_str()),
            "the uploaded metadata's source_url must be the resolved detail \
             page URL, so the citation can link back to the real document"
        );
    }

    #[tokio::test]
    async fn should_only_ingest_rows_newer_than_the_bookmark_on_second_run() {
        let server = MockServer::start().await;
        let src = server.uri();

        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(listing_html(&[("75", "20/07/2026"), ("74", "13/07/2026")])),
            )
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/detail/75"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(detail_html("75", "/attachment/75.txt")),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/attachment/75.txt"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Contenuto dell'atto 75."))
            .mount(&server)
            .await;

        let store = temp_store().await;
        let section = store
            .upsert_section(kb_store::NewIngestSection {
                name: "delibere".into(),
                ordering: 0,
            })
            .await
            .expect("section insert failed");
        store
            .upsert_bookmark(section.id, &src, "74", "2026-07-13")
            .await
            .expect("bookmark seed failed");

        let upload = Arc::new(RecordingUploadPort::new());
        let adapter = HalleyCurationAdapter::new(store, upload.clone());

        let response = adapter
            .ingest("delibere", &src, RecencyWindow::Days(30))
            .await
            .expect("curation failed");

        assert_eq!(response.status, "ingested 1 document(s)");
        let calls = upload.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].2, "delibera-di-giunta-75-2026-07-20.txt");
    }

    #[tokio::test]
    async fn should_not_advance_bookmark_past_a_row_whose_upload_fails() {
        let server = MockServer::start().await;
        let src = server.uri();

        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(listing_html(&[("75", "20/07/2026"), ("74", "13/07/2026")])),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/index/table-delibere-public-page/2"))
            .respond_with(ResponseTemplate::new(200).set_body_string(listing_html(&[])))
            .mount(&server)
            .await;

        for number in ["75", "74"] {
            Mock::given(method("GET"))
                .and(path(format!("/detail/{number}")))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_string(detail_html(number, &format!("/attachment/{number}.txt"))),
                )
                .mount(&server)
                .await;
            Mock::given(method("GET"))
                .and(path(format!("/attachment/{number}.txt")))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_string(format!("Contenuto dell'atto numero {number}.")),
                )
                .mount(&server)
                .await;
        }

        let store = temp_store().await;
        let section = store
            .upsert_section(kb_store::NewIngestSection {
                name: "delibere".into(),
                ordering: 0,
            })
            .await
            .expect("section insert failed");

        // Fails specifically on the OLDEST row (74), which is curated first
        // (oldest-to-newest processing order) — the newest row (75) should
        // never even be attempted, and no bookmark should be written at all
        // since nothing succeeded before the failure.
        let upload = Arc::new(RecordingUploadPort::failing_on(
            "delibera-di-giunta-74-2026-07-13.txt",
        ));
        let adapter = HalleyCurationAdapter::new(store.clone(), upload.clone());

        let result = adapter
            .ingest("delibere", &src, RecencyWindow::Days(30))
            .await;
        assert!(result.is_err());

        let bookmark = store
            .get_bookmark(section.id, &src)
            .await
            .expect("query failed");
        assert!(
            bookmark.is_none(),
            "no row succeeded before the failure, so no bookmark should be written"
        );
        assert!(upload.calls.lock().unwrap().is_empty());
    }

    struct HangingAfterFirstUploadPort {
        calls: Mutex<Vec<String>>,
        reached_second: Arc<tokio::sync::Notify>,
    }

    impl HangingAfterFirstUploadPort {
        fn new(reached_second: Arc<tokio::sync::Notify>) -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                reached_second,
            }
        }
    }

    #[async_trait]
    impl UploadPort for HangingAfterFirstUploadPort {
        async fn ingest_uploaded(
            &self,
            _text: &str,
            _section: &str,
            filename: &str,
            _metadata: &UploadMetadata,
        ) -> Result<Vec<i64>, crate::admin::upload::UploadError> {
            let is_first = self.calls.lock().unwrap().is_empty();
            self.calls.lock().unwrap().push(filename.to_string());
            if is_first {
                return Ok(vec![1]);
            }
            // Second item onward: signal we got here, then hang forever —
            // standing in for the request's connection being cut mid-flight
            // (exactly what a client-side timeout does to an in-flight axum
            // handler future: it gets dropped wherever it currently is,
            // running no further code, including whatever would have run
            // "after" this await).
            self.reached_second.notify_one();
            std::future::pending::<()>().await;
            unreachable!()
        }
    }

    #[tokio::test]
    async fn should_checkpoint_the_bookmark_after_every_successfully_curated_item_not_only_at_batch_end()
     {
        // Regression test for a real bug found running this against the live
        // site: a client-side timeout killed the curl client after row 74
        // had already been durably chunked and embedded, but row 75 was
        // still in flight. Dropping the `ingest()` future at that point
        // (exactly what axum does when the client disconnects mid-request)
        // skipped every bookmark write that only happened at full-batch-end
        // or on an explicit `Err` — losing all progress even though 74 had
        // genuinely succeeded. The bookmark must be durably written
        // immediately after each item succeeds, not batched to the end.
        let server = MockServer::start().await;
        let src = server.uri();

        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(listing_html(&[("75", "20/07/2026"), ("74", "13/07/2026")])),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/index/table-delibere-public-page/2"))
            .respond_with(ResponseTemplate::new(200).set_body_string(listing_html(&[])))
            .mount(&server)
            .await;

        for number in ["74", "75"] {
            Mock::given(method("GET"))
                .and(path(format!("/detail/{number}")))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_string(detail_html(number, &format!("/attachment/{number}.txt"))),
                )
                .mount(&server)
                .await;
            Mock::given(method("GET"))
                .and(path(format!("/attachment/{number}.txt")))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_string(format!("Contenuto dell'atto numero {number}.")),
                )
                .mount(&server)
                .await;
        }

        let store = temp_store().await;
        let section = store
            .upsert_section(kb_store::NewIngestSection {
                name: "delibere".into(),
                ordering: 0,
            })
            .await
            .expect("section insert failed");

        let reached_second = Arc::new(tokio::sync::Notify::new());
        let upload = Arc::new(HangingAfterFirstUploadPort::new(reached_second.clone()));
        let adapter = HalleyCurationAdapter::new(store.clone(), upload);

        {
            let ingest_fut = adapter.ingest("delibere", &src, RecencyWindow::Days(30));
            tokio::pin!(ingest_fut);
            tokio::select! {
                _ = &mut ingest_fut => panic!(
                    "ingest() must not complete — the second item's upload hangs forever"
                ),
                _ = reached_second.notified() => {
                    // Row 74 (processed first, oldest-to-newest) has already
                    // succeeded by now; row 75's upload is stuck mid-flight.
                }
            }
            // Dropping `ingest_fut` here (end of this scope) simulates the
            // client disconnecting: the future is cancelled wherever it
            // currently is, exactly like axum would on a real timeout.
        }

        let bookmark = store
            .get_bookmark(section.id, &src)
            .await
            .expect("query failed")
            .expect(
                "row 74 already succeeded before the future was cancelled mid-row-75 — \
                 the bookmark must reflect that progress, not be missing entirely",
            );
        assert_eq!(
            bookmark.last_item_ref, "74",
            "bookmark must checkpoint row 74 immediately after it succeeds, proving \
             progress is saved incrementally rather than only at full-batch end"
        );
    }

    #[test]
    fn should_build_unique_filename_from_act_type_number_and_date() {
        let row = HalleyListingRow {
            act_type: "Delibera Di Giunta".to_string(),
            number: "74".to_string(),
            date: chrono::NaiveDate::from_ymd_opt(2026, 7, 13).unwrap(),
            title: "irrelevant".to_string(),
            detail_path: "/irrelevant".to_string(),
        };
        let filename =
            HalleyCurationAdapter::unique_filename(&row, "delibera copia uso amministrativo.pdf");
        assert_eq!(filename, "delibera-di-giunta-74-2026-07-13.pdf");
    }

    #[tokio::test]
    async fn should_not_collapse_distinct_acts_sharing_the_same_generic_attachment_filename() {
        // Regression test for a real bug found running this against the
        // live site: Halley reuses the exact same generic attachment
        // filename ("delibera copia uso amministrativo.pdf") across many
        // unrelated acts. Using it verbatim as `source_ref` merged dozens
        // of genuinely different delibere into one indistinguishable,
        // uncitable 178-chunk blob before this fix.
        let server = MockServer::start().await;
        let src = server.uri();

        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(listing_html(&[("75", "20/07/2026"), ("74", "13/07/2026")])),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/index/table-delibere-public-page/2"))
            .respond_with(ResponseTemplate::new(200).set_body_string(listing_html(&[])))
            .mount(&server)
            .await;

        for number in ["75", "74"] {
            // Both rows' detail pages advertise the SAME generic attachment
            // filename text — the real-world Halley quirk — even though the
            // underlying attachment URL differs per act, same as real life.
            Mock::given(method("GET"))
                .and(path(format!("/detail/{number}")))
                .respond_with(ResponseTemplate::new(200).set_body_string(
                    detail_html_with_filename(
                        number,
                        &format!("/attachment/{number}.txt"),
                        "delibera copia uso amministrativo.txt",
                    ),
                ))
                .mount(&server)
                .await;
            Mock::given(method("GET"))
                .and(path(format!("/attachment/{number}.txt")))
                .respond_with(
                    ResponseTemplate::new(200).set_body_string(format!("Contenuto atto {number}.")),
                )
                .mount(&server)
                .await;
        }

        let store = temp_store().await;
        store
            .upsert_section(kb_store::NewIngestSection {
                name: "delibere".into(),
                ordering: 0,
            })
            .await
            .expect("section insert failed");

        let upload = Arc::new(RecordingUploadPort::new());
        let adapter = HalleyCurationAdapter::new(store, upload.clone());

        adapter
            .ingest("delibere", &src, RecencyWindow::Days(30))
            .await
            .expect("curation failed");

        let calls = upload.calls.lock().unwrap();
        assert_eq!(calls.len(), 2);
        let filenames: std::collections::HashSet<_> =
            calls.iter().map(|(_, _, f, _, _)| f).collect();
        assert_eq!(
            filenames.len(),
            2,
            "each act must get a distinct source_ref, even when Halley's own \
             attachment filename is identical across both: {filenames:?}"
        );
    }
}

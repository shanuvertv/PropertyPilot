//! Renewal notice letters and reports as PDF via Typst (PLAN.md D8).

use serde::{Deserialize, Serialize};
use typst::foundations::{Array, Dict, Str, Value};
use typst_as_lib::typst_kit_options::TypstKitFontOptions;
use typst_as_lib::TypstEngine;
use typst_layout::PagedDocument;

use crate::error::{ServiceError, ServiceResult};

/// Letterhead block kept in Settings (`org.letterhead`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Letterhead {
    pub company_name: String,
    #[serde(default)]
    pub address_lines: Vec<String>,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub footer: String,
}

impl Default for Letterhead {
    fn default() -> Self {
        Self {
            company_name: "Leasing Department".into(),
            address_lines: vec![],
            phone: String::new(),
            email: String::new(),
            footer: String::new(),
        }
    }
}

pub struct Letter<'a> {
    pub letterhead: &'a Letterhead,
    pub date: &'a str,
    pub reference: &'a str,
    pub recipient_lines: Vec<String>,
    pub subject: &'a str,
    pub body_text: &'a str,
    pub signature_lines: Vec<String>,
}

const LETTER_TEMPLATE: &str = r##"
#import sys: inputs
#let d = inputs
#set page(paper: "a4", margin: (x: 2.2cm, top: 2cm, bottom: 2.2cm), footer: context [
  #set text(size: 8.5pt, fill: luma(110))
  #d.footer
  #h(1fr)
  #counter(page).display("1 / 1", both: true)
])
#set text(font: ("Segoe UI", "Arial", "Liberation Sans", "DejaVu Sans", "Libertinus Serif"), size: 10.5pt)
#set par(justify: false, leading: 0.6em)

#grid(columns: (1fr, auto), align: (left, right),
  [
    #text(size: 15pt, weight: "bold", fill: rgb("#146C68"))[#d.company]
    #v(2pt)
    #text(size: 9pt, fill: luma(90))[#d.address_lines.join(" · ")]
    #if d.contact != "" [ #linebreak() #text(size: 9pt, fill: luma(90))[#d.contact] ]
  ],
  [
    #text(size: 9.5pt)[#d.date]
    #linebreak()
    #text(size: 9pt, fill: luma(90))[Ref: #d.reference]
  ],
)
#v(4pt)
#line(length: 100%, stroke: 0.6pt + rgb("#146C68"))
#v(1.1cm)

#for l in d.recipient_lines [ #l \ ]
#v(0.6cm)
#text(weight: "bold")[Subject: #d.subject]
#v(0.4cm)
#for p in d.paragraphs [
  #p.join(linebreak())
  #v(0.32cm)
]
#v(0.6cm)
#for l in d.signature_lines [ #l \ ]
"##;

fn s(v: &str) -> Value {
    Value::Str(Str::from(v))
}

fn arr(items: &[String]) -> Value {
    Value::Array(items.iter().map(|x| s(x)).collect::<Array>())
}

/// Renders the letter to PDF bytes. Fonts come from the host (Segoe UI / Arial on
/// Windows) with Typst's embedded fallbacks, so output never depends on files on disk.
pub fn render_letter(letter: &Letter<'_>) -> ServiceResult<Vec<u8>> {
    // Paragraphs are blank-line separated; single line breaks inside one are kept.
    let paragraphs: Vec<Vec<String>> = letter
        .body_text
        .replace("\r\n", "\n")
        .split("\n\n")
        .map(|p| {
            p.lines()
                .map(|l| l.trim().to_owned())
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|p| !p.is_empty())
        .collect();
    let contact = [
        letter.letterhead.phone.as_str(),
        letter.letterhead.email.as_str(),
    ]
    .iter()
    .filter(|x| !x.is_empty())
    .cloned()
    .collect::<Vec<_>>()
    .join(" · ");

    let mut dict = Dict::new();
    dict.insert("company".into(), s(&letter.letterhead.company_name));
    dict.insert(
        "address_lines".into(),
        arr(&letter.letterhead.address_lines),
    );
    dict.insert("contact".into(), s(&contact));
    dict.insert("footer".into(), s(&letter.letterhead.footer));
    dict.insert("date".into(), s(letter.date));
    dict.insert("reference".into(), s(letter.reference));
    dict.insert("recipient_lines".into(), arr(&letter.recipient_lines));
    dict.insert("subject".into(), s(letter.subject));
    dict.insert(
        "paragraphs".into(),
        Value::Array(paragraphs.iter().map(|p| arr(p)).collect::<Array>()),
    );
    dict.insert("signature_lines".into(), arr(&letter.signature_lines));

    let engine = TypstEngine::builder()
        .main_file(LETTER_TEMPLATE)
        .search_fonts_with(TypstKitFontOptions::default())
        .build();
    let doc: PagedDocument = engine
        .compile_with_input(dict)
        .output
        .map_err(|e| ServiceError::Internal(format!("typst: {e}")))?;
    typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default())
        .map_err(|e| ServiceError::Internal(format!("pdf: {e:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_a_letter_pdf() {
        let lh = Letterhead {
            company_name: "Al Noor Properties".into(),
            address_lines: vec!["P.O. Box 1234".into(), "Dubai".into()],
            phone: "+971 4 000 0000".into(),
            email: "leasing@example.com".into(),
            footer: "Issued under the tenancy contract terms.".into(),
        };
        let letter = Letter {
            letterhead: &lh,
            date: "11 September 2026",
            reference: "C-0001",
            recipient_lines: vec![
                "Falcon Trading LLC".into(),
                "Attn: Sara Haddad".into(),
                "Unit 101, Al Noor Tower".into(),
            ],
            subject: "Renewal of tenancy contract C-0001",
            body_text: "Dear Sara,\n\nWe refer to your contract.\n\nKind regards,\nLeasing Team",
            signature_lines: vec!["Umair Admin".into(), "Al Noor Properties".into()],
        };
        let pdf = render_letter(&letter).expect("pdf");
        assert!(pdf.starts_with(b"%PDF"), "not a pdf");
        assert!(pdf.len() > 1000);
    }
}

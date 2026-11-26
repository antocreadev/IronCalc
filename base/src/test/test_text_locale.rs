#![allow(clippy::unwrap_used)]

// TEXT reads its format code as it is written in the workbook's locale, as Excel does with the
// regional settings: the locale's decimal and group separators, and its date letters.
// French, German and Italian codes are those of each language's "Number format codes" page on
// support.microsoft.com (French: aaaa, jj, Standard; German: JJJJ, TT; Italian: aaaa, gg).

use crate::Model;

fn new_model(locale: &'static str) -> Model<'static> {
    Model::new_empty("model", locale, "UTC", locale).unwrap()
}

#[test]
fn french_decimal_and_group_separators() {
    let mut model = new_model("fr");
    model._set("A1", "=TEXTE(1234,5;\"0,00\")");
    model._set("A2", "=TEXTE(1234,5;\"# ##0,00\")");
    model._set("A3", "=TEXTE(0,25;\"0,0 %\")");
    model._set("A4", "=TEXTE(1234,5;\"Standard\")");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"1234,50");
    assert_eq!(model._get_text("A2"), *"1\u{202f}234,50");
    assert_eq!(model._get_text("A3"), *"25,0 %");
    assert_eq!(model._get_text("A4"), *"1234,5");
}

#[test]
fn french_date_letters() {
    let mut model = new_model("fr");
    model._set("A1", "=DATE(2026;11;26)");
    model._set("B1", "=TEXTE(A1;\"jj/mm/aaaa\")");
    model._set("B2", "=TEXTE(A1;\"jjjj j mmmm aa\")");
    model._set("B3", "=TEXTE(A1;\"jj.mm.aaaa\")");
    model._set("B4", "=TEXTE(0,5;\"hh:mm:ss,00\")");
    model.evaluate();

    assert_eq!(model._get_text("B1"), *"26/11/2026");
    assert_eq!(model._get_text("B2"), *"jeudi 26 novembre 26");
    assert_eq!(model._get_text("B3"), *"26.11.2026");
    assert_eq!(model._get_text("B4"), *"12:00:00,00");
}

#[test]
fn french_keeps_english_date_letters_and_quoted_text() {
    let mut model = new_model("fr");
    model._set("A1", "=DATE(2026;11;26)");
    model._set("B1", "=TEXTE(A1;\"dd/mm/yyyy\")");
    model._set("B2", "=TEXTE(5;\"0 \"\"j,a.\"\"\")");
    model._set("B3", "=TEXTE(0,75;\"h:mm AM/PM\")");
    model.evaluate();

    assert_eq!(model._get_text("B1"), *"26/11/2026");
    assert_eq!(model._get_text("B2"), *"5 j,a.");
    assert_eq!(model._get_text("B3"), *"6:00 PM");
}

#[test]
fn german_format_codes() {
    let mut model = new_model("de");
    model._set("A1", "=TEXT(1234,5;\"#.##0,00\")");
    model._set("A2", "=TEXT(DATUM(2026;11;26);\"TT.MM.JJJJ\")");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"1.234,50");
    assert_eq!(model._get_text("A2"), *"26.11.2026");
}

#[test]
fn italian_format_codes() {
    let mut model = new_model("it");
    model._set("A1", "=TESTO(1234,5;\"#.##0,00\")");
    model._set("A2", "=TESTO(DATA(2026;11;26);\"gg/mm/aaaa\")");
    model._set("A3", "=TESTO(1234,5;\"Generale\")");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"1.234,50");
    assert_eq!(model._get_text("A2"), *"26/11/2026");
    assert_eq!(model._get_text("A3"), *"1234,5");
}

#[test]
fn english_format_codes_are_unchanged() {
    let mut model = new_model("en");
    model._set("A1", "=TEXT(1234.5,\"#,##0.00\")");
    model._set("A2", "=TEXT(DATE(2026,11,26),\"dd/mm/yyyy\")");
    model._set("A3", "=TEXT(12200000,\"#,##0.0,\")");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"1,234.50");
    assert_eq!(model._get_text("A2"), *"26/11/2026");
    assert_eq!(model._get_text("A3"), *"12,200.0");
}

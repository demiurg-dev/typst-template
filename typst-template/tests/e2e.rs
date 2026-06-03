//! End-to-end: prove the whole chain works — a derived Rust struct becomes
//! `sys.inputs`, the document compiles, and the injected values appear in the
//! rendered layout (and the PDF bytes).

use typst_template::typst::layout::{Frame, FrameItem};
use typst_template::{ToDict, WorldBaseConfig};

#[derive(ToDict)]
#[typst(rename_all = "camelCase")]
struct Invoice {
    client_name: String,
    amount: i64,
}

/// Concatenates every text run in a frame (recursing into groups).
fn collect_text(frame: &Frame, out: &mut String) {
    for (_, item) in frame.items() {
        match item {
            FrameItem::Text(text) => out.push_str(text.text.as_str()),
            FrameItem::Group(group) => collect_text(&group.frame, out),
            _ => {}
        }
    }
}

#[test]
fn inputs_reach_the_rendered_document() {
    let base = WorldBaseConfig::new(".").system_fonts(false).build();
    let world = base
        .concrete("main.typ")
        .file("main.typ", "#sys.inputs.clientName: #sys.inputs.amount EUR")
        .inputs(Invoice { client_name: "ACME".into(), amount: 1234 })
        .build();

    // Compile and inspect the laid-out document.
    let document = world.compile().output.expect("document compiles");
    assert_eq!(document.pages.len(), 1);

    let mut text = String::new();
    for page in &document.pages {
        collect_text(&page.frame, &mut text);
    }
    assert!(text.contains("ACME"), "client name missing from rendered text: {text:?}");
    assert!(text.contains("1234"), "amount missing from rendered text: {text:?}");

    // And the same world exports to a structurally valid PDF.
    let pdf = world.render_pdf_default().output.expect("pdf exports");
    assert!(pdf.starts_with(b"%PDF"));
}

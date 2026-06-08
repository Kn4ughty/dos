extern crate proc_macro;

use core::iter::Iterator;

use proc_macro::TokenStream;
use quote::quote;

#[derive(Debug, PartialEq, Eq)]
enum Segment {
    Plain(String),
    /// text, colour name
    Coloured(String, String),
}

// Fully qualified paths _not_ used here so that it can later be integrated with serial colours
fn colour_ident(name: &str) -> proc_macro2::TokenStream {
    match name {
        "black" => quote! { Colour::Black },
        "white" => quote! { Colour::White },
        "red" => quote! { Colour::Red },
        "blue" => quote! { Colour::Blue },
        other => panic!("Unknown colour tag <{other}"),
    }
}

#[must_use]
fn parse_segment(s: &str) -> Vec<Segment> {
    // To support nesting this will need to be recursive

    let mut out = Vec::new();

    let mut remaining = s;

    while !remaining.is_empty() {
        if let Some(tag_start) = remaining.find('<') {
            if remaining[tag_start + 1..].starts_with('<') {
                // Closing literal
                println!("< literal");
                out.push(Segment::Plain(remaining[..tag_start + 2].to_string()));
                remaining = &remaining[tag_start + 2..];
                continue;
            } else if tag_start != 0 {
                // Push as plain string
                out.push(Segment::Plain(remaining[..tag_start + 1].to_string()));
                remaining = &remaining[tag_start + 1..];
            }

            let tag_end = remaining.find('>').expect("unclosed tag");
            // remaining has been trimed at this point
            let colour = &remaining[1..tag_end];
            // Consume the colour
            remaining = &remaining[tag_end + 1..];
            println!("r: {remaining}");
            println!("c: {colour}");

            // find closing tag
            let close_start = remaining.find("</").expect("Closing tag");
            let content = &remaining[..close_start];

            remaining = &remaining[close_start + 1..];

            let close_end = remaining.find('>').expect("unclosed tag");

            let close_colour = &remaining[1..close_end];

            assert_eq!(colour, close_colour);

            out.push(Segment::Coloured(colour.to_string(), content.to_string()));
            remaining = &remaining[close_end + 1..];
        } else {
            out.push(Segment::Plain(remaining.to_string()));
            remaining = "";
        }
    }

    out
}

#[test]
fn test_parse_segment() {
    assert_eq!(
        parse_segment("plain"),
        vec![Segment::Plain("plain".to_string())]
    );

    let out = parse_segment("<red>Red text</red>");
    assert_eq!(
        out,
        vec![Segment::Coloured("red".to_string(), "Red text".to_string())]
    );
}

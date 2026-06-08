extern crate proc_macro;
use syn::{Expr, LitStr, parse_macro_input, token::Group};

use core::iter::Iterator;

use proc_macro::TokenStream;
use quote::quote;

#[proc_macro]
pub fn cprint(input: TokenStream) -> TokenStream {
    let lit = parse_macro_input!(input as LitStr);
    let s = lit.value();
    println!("{s:?}");

    let segments = parse_segments(s.as_str());
    println!("{segments:?}");

    let calls = segments.iter().map(|segment| {
        match segment {
            Segment::Plain(s) => {
                quote! { os::vga_buffer::_print(format_args!("{}", #s) ) }
            },
            Segment::Coloured(c, s) => {
                let colour_expr = colour_ident(c.as_str());
                quote! { os::vga_buffer::_print_coloured(format_args!("{}", #s), #colour_expr, os::vga_buffer::Colour::Black ) }
            }
        }
    });

    // "\"\"".parse().unwrap()
    quote! { { #(#calls)* } }.into()
}

#[derive(Debug, PartialEq, Eq)]
enum Segment {
    Plain(String),
    /// text, colour name
    Coloured(String, String),
}

// Fully qualified paths _not_ used here so that it can later be integrated with serial colours
fn colour_ident(name: &str) -> proc_macro2::TokenStream {
    match name {
        "black" => quote! { os::vga_buffer::Colour::Black },
        "blue" => quote! { os::vga_buffer::Colour::Blue},
        "green" => quote! { os::vga_buffer::Colour::Green},
        "cyan" => quote! { os::vga_buffer::Colour::Cyan},
        "red" => quote! { os::vga_buffer::Colour::Red},
        "magenta" => quote! { os::vga_buffer::Colour::Magenta},
        "brown" => quote! { os::vga_buffer::Colour::Brown},
        "lgray" => quote! { os::vga_buffer::Colour::LightGray},
        "dgray" => quote! { os::vga_buffer::Colour::DarkGray},
        "lblue" => quote! { os::vga_buffer::Colour::LightBlue},
        "lgreen" => quote! { os::vga_buffer::Colour::LightGreen},
        "lcyan" => quote! { os::vga_buffer::Colour::LightCyan},
        "lred" => quote! { os::vga_buffer::Colour::LightRed},
        "pink" => quote! { os::vga_buffer::Colour::Pink},
        "yellow" => quote! { os::vga_buffer::Colour::Yellow},
        "white" => quote! { os::vga_buffer::Colour::White},
        other => panic!("Unknown colour tag {other}"),
    }
}

#[must_use]
fn parse_segments(s: &str) -> Vec<Segment> {
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
                out.push(Segment::Plain(remaining[..tag_start].to_string()));
                remaining = &remaining[tag_start..];
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
        parse_segments("plain"),
        vec![Segment::Plain("plain".to_string())]
    );

    assert_eq!(
        parse_segments("<red>Red text</red>"),
        vec![Segment::Coloured("red".to_string(), "Red text".to_string())]
    );

    assert_eq!(
        parse_segments("<blue> BLUE </blue> regular <red>Red text</red>"),
        vec![
            Segment::Coloured("blue".to_string(), " BLUE ".to_string()),
            Segment::Plain(" regular ".to_string()),
            Segment::Coloured("red".to_string(), "Red text".to_string())
        ]
    );
}

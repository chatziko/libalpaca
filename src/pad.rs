//! Contains padding functions for different resource types.
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use std::iter::Extend;

use objects::*;

static CSS_COMMENT_START: &'static str = "/*";
const CSS_COMMENT_START_SIZE: usize = 2;
static CSS_COMMENT_END: &'static str = "*/";
const CSS_COMMENT_END_SIZE: usize = 2;
static HTML_COMMENT_START: &'static str = "<!--";
const HTML_COMMENT_START_SIZE: usize = 4;
static HTML_COMMENT_END: &'static str = "-->";
const HTML_COMMENT_END_SIZE: usize = 3;

/// Pads an html to its target size.
pub fn get_html_padding(html: &mut Object, target_size: usize) {
    let pad_len = target_size - html.content.len();
    let pad_len = pad_len - HTML_COMMENT_START_SIZE - HTML_COMMENT_END_SIZE;
    let mut pad = Vec::from(HTML_COMMENT_START);
    add_random_chars(&mut pad, pad_len);
    pad.extend(Vec::from(HTML_COMMENT_END));
    html.content.extend(pad);
}


/// Pads an object to its target size.
pub fn get_object_padding(object: &mut Object, size: usize, target_size: usize) {
    let pad_len = target_size - size;
    let padding;
    match object.kind {
        ObjectKind::CSS => {
            if size + 4 > target_size {
                // Consider the 4 additional comment-bytes.
                return;
            }
            padding = get_css_padding(pad_len);
        }
        _ => padding = get_binary_padding(pad_len),
    };

    object.content.extend(padding);
}

fn get_css_padding(pad_len: usize) -> Vec<u8> {
    let pad_len = pad_len - CSS_COMMENT_START_SIZE - CSS_COMMENT_END_SIZE;
    let mut pad = Vec::from(CSS_COMMENT_START);
    add_random_chars(&mut pad, pad_len);
    pad.extend(Vec::from(CSS_COMMENT_END));
    pad
}

fn add_random_chars(pad: &mut Vec<u8>, pad_len: usize) {
    let mut rng = thread_rng();
    for _ in 0..pad_len {
        pad.push(rng.sample(Alphanumeric) as u8);
    }
}

fn get_binary_padding(pad_len: usize) -> Vec<u8> {
    let mut rng = thread_rng();
    let mut pad: Vec<u8> = Vec::with_capacity(pad_len);
    for _ in 0..pad_len {
        pad.push(rng.gen::<u8>());
    }
    pad
}

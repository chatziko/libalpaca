//! Contains main morphing routines.
use std::ffi::CStr;
use pad::{get_html_padding, get_object_padding};
use dom;
use dom::{Object,ObjectKind};
use distribution::{Distributions, sample_ge, sample_ge_many};
use deterministic::*;
use aux::stringify_error;

use kuchiki::NodeRef;

#[repr(C)]
pub struct MorphInfo {
    // request info
    content: *const u8,     // u8 = uchar
    size: usize,
    root: *const u8,
    uri: *const u8,
    http_host: *const u8,
    alias: usize,
    query: *const u8,       // part after ?
    content_type: *const u8,

    probabilistic: usize,   // boolean

    // for probabilistic
    dist_html_size: *const u8,
    dist_obj_number: *const u8,
    dist_obj_size: *const u8,

    // for deterministic
    obj_num: usize,
    obj_size: usize,
    max_obj_size: usize,
}

/// It samples a new page using probabilistic morphing, changes the
/// references to its objects accordingly, and pads it.
#[no_mangle]
pub extern "C" fn morph_html(pinfo: *mut MorphInfo) -> u8 {

    
    std::env::set_var("RUST_BACKTRACE", "full");
    let info = unsafe { &mut *pinfo };

    let root = c_string_to_str(info.root).unwrap();
    let uri = c_string_to_str(info.uri).unwrap();
    let http_host = c_string_to_str(info.http_host).unwrap();

    // /* Convert arguments into &str */
    let html = match c_string_to_str(info.content) {
        Ok(s) => s,
        Err(e) => {
            eprint!("libalpaca: cannot read html content of {}: {}\n", uri, e);
            return 0;       // return NULL pointer if html cannot be converted to a string
        }
    };

    let document = dom::parse_html(html);

    let full_root = String::from(root).replace("$http_host", http_host);

    let mut objects = dom::parse_objects(&document, full_root.as_str(), uri, info.alias); // Vector of objects found in the html.
    let orig_n = objects.len(); // Number of original objects.

    let mut dists: Option<Distributions> = None;

    if info.probabilistic != 0 {
        // Probabilistic alpaca
        // Construct a Distributions object containing the given distributions.
        let dist_html_size = c_string_to_str(info.dist_html_size).unwrap();
        let dist_obj_number = c_string_to_str(info.dist_obj_number).unwrap();
        let dist_obj_size = c_string_to_str(info.dist_obj_size).unwrap();

        dists = match Distributions::from(dist_html_size, dist_obj_number, dist_obj_size) {
            Ok(result) => Some(result),
            Err(e) => {
                eprint!("libalpace: cannot load distributions: {}\n", e);
                return document_to_c(&document, info);
            }
        };

        match morph_from_distribution(&mut objects, dists.as_ref().unwrap()) {
            Ok(_) => {},
            Err(e) => {
                eprint!("libalpaca: morph_from_distribution failed: {}\n", e);
                return document_to_c(&document, info);
            }
        }

    } else {
        // Deterministic alpaca
        match morph_deterministic(&mut objects, info.obj_num, info.obj_size, info.max_obj_size) {
            Ok(_) => {},
            Err(e) => {
                eprint!("libalpaca: cannot morph_deterministic: {}\n", e);
                return document_to_c(&document, info);
            },
        }
    }

    match insert_objects_refs(&document, &objects, orig_n) {
        Ok(_) => {},
        Err(e) => {
            eprint!("libalpaca: insert_objects_refs failed: {}\n", e);
            return document_to_c(&document, info);
        }
    }

    let mut content = dom::serialize_html(&document);

    // find target size
    let html_min_size = content.len() + 7; // Plus 7 because of the comment characters.
    let target_size =
        if info.probabilistic != 0 {
            match sample_ge(&(dists.unwrap().html), html_min_size) {
                Ok(size) => size,
                Err(e) => {
                    eprint!("libalpaca: cannot sample html page size: {}\n", e);
                    return document_to_c(&document, info);
                }
            }
        } else {
            // Target size for the html is a multiple of "obj_size".
            get_multiple(info.obj_size, html_min_size)
        };

    get_html_padding(&mut content, target_size); // Pad the html to the target size.

    return content_to_c(content, info);
}

/// Returns the object's padding.
#[no_mangle]
pub extern "C" fn morph_object(pinfo: *mut MorphInfo) -> u8 {

    let info = unsafe { &mut *pinfo };

    let content_type = c_string_to_str(info.content_type).unwrap();
    let query = c_string_to_str(info.query).unwrap();

    let kind = dom::parse_object_kind(content_type);

    let target_size = dom::parse_target_size(query);
    if (target_size == 0) || (target_size <= info.size) {
        // Target size has to be greater than current size.
        return content_to_c(Vec::new(), info);
    }

    let padding = get_object_padding(kind, info.size, target_size); // Get the padding for the object.

    return content_to_c(padding, info);
}

/// Frees memory allocated in rust.
#[no_mangle]
pub extern "C" fn free_memory(data: *mut u8, size: usize) {

    let s = unsafe { std::slice::from_raw_parts_mut(data, size) };
    let s = s.as_mut_ptr();
    unsafe {
        Box::from_raw(s);
    }
}

fn morph_from_distribution(
    objects: &mut Vec<Object>,
    dists: &Distributions,
) -> Result<(), String> {
    // we'll have at least as many objects as the original ones
    let initial_obj_no = objects.len();

    // Sample target number of objects (count)
    let target_count = match sample_ge(&dists.obj_num, initial_obj_no) {
        Ok(c) => c,
        Err(e) => {
            eprint!("libalpaca: could not sample object number ({}), leaving unchanged ({})\n", e, initial_obj_no);
            initial_obj_no
        }
    };

    // To more closely match the actual obj_size distribution, we'll sample values for all objects,
    // And then we'll use the largest to pad existing objects and the smallest for padding objects.
    let mut target_sizes: Vec<usize> = sample_ge_many(&dists.obj_size, 1, target_count)?;
    target_sizes.sort_unstable();       // ascending

    // Pad existing objects
    for obj in &mut *objects {
        let needed_size = obj.content.len() +
            match obj.kind { ObjectKind::CSS | ObjectKind::JS => 4, _ => 0 };   // CSS/JS padding needs to be at least 4.

        // Take the largest size, if not enough draw a new one with this specific needed_size
        obj.target_size = if target_sizes[target_sizes.len()-1] >= needed_size {
            Some(target_sizes.pop().unwrap())
        } else {
            match sample_ge(&dists.obj_size, needed_size) {
                Ok(size) => Some(size),
                Err(e) => {
                    eprint!("libalpaca: warning: no padding was found for {} ({})\n", obj.uri, e);
                    None
                },
            }
        };
    }

    // create padding objects, using the smallest of the sizes
    for i in 0..target_count - initial_obj_no {
        objects.push(Object::padding(target_sizes[i]));
    }

    Ok(())
}

fn morph_deterministic(
    objects: &mut Vec<Object>,
    obj_num: usize,
    obj_size: usize,
    max_obj_size: usize,
) -> Result<(), String> {
    // we'll have at least as many objects as the original ones
    let initial_obj_no = objects.len();

    // Sample target number of objects (count) and target sizes for morphed
    // objects. Count is a multiple of "obj_num" and bigger than "min_count".
    // Target size for each objects is a multiple of "obj_size" and bigger
    // than the object's  original size.
    let target_count = get_multiple(obj_num, initial_obj_no);

    for i in 0..objects.len() {
        let min_size = objects[i].content.len()
            + match objects[i].kind { ObjectKind::CSS | ObjectKind::JS => 4, _ => 0 };

        let obj_target_size = get_multiple(obj_size, min_size);
        objects[i].target_size = Some(obj_target_size);
    }

    let fake_objects_count = target_count - initial_obj_no; // The number of fake objects.

    // To get the target size of each fake object, sample uniformly a multiple
    // of "obj_size" which is smaller than "max_obj_size".
    let fake_objects_sizes = get_multiples_in_range(obj_size, max_obj_size, fake_objects_count)?;

    // Add the fake objects to the vector.
    for i in 0..fake_objects_count {
        objects.push(Object::padding(fake_objects_sizes[i]));
    }

    Ok(())
}

/// Inserts the ALPaCA GET parameters to the html objects, and adds the fake objects to the html.
fn insert_objects_refs(document: &NodeRef, objects: &[Object], n: usize) -> Result<(), String> {
    let init_obj = &objects[0..n]; // Slice which contains initial objects
    let padding_obj = &objects[n..]; // Slice which contains ALPaCA objects

    for object in init_obj {
        // ignore objects without target size
        if !object.target_size.is_none() {
            append_ref(&object);
        }
    }

    add_padding_objects(&document, padding_obj);

    Ok(())
}

/// Appends the ALPaCA GET parameter to an html element
fn append_ref(object: &Object) {
    // Construct the link with the appended new parameter
    let mut new_link = String::from("alpaca-padding=");
    new_link.push_str(&(object.target_size.unwrap().to_string())); // Append the target size

    let node = object.node.as_ref().unwrap();
    let attr = match node.as_element().unwrap().name.local.to_lowercase().as_ref() {
        "img" | "script" => "src",
        "link" => "href",
        _ => panic!("shouldn't happen"),
    };

    // Check if there is already a GET parameter in the file path
    let prefix = if object.uri.contains("?") { '&' } else { '?' };

    new_link.insert(0, prefix);
    new_link.insert_str(0, &object.uri);

    dom::node_set_attribute(node, attr, new_link);
}

/// Adds the fake ALPaCA objects in the end of the html body
fn add_padding_objects(document: &NodeRef, objects: &[Object]) {

    // append the objects either to the <body> tag, if exists, otherwise
    // to the whole document
    let node_data;  // to outlive the match
    let node = match document.select("body").unwrap().next() {
        Some(nd) => { node_data = nd; node_data.as_node() },
        None => document,
    };

    for object in objects {
        let elem = dom::create_element("img");
        dom::node_set_attribute(&elem, "src", format!("/__alpaca_fake_image.png?alpaca-padding={}", object.target_size.unwrap()));
        dom::node_set_attribute(&elem, "style", String::from("visibility:hidden"));
        node.append(elem);
    }
}

// Builds the returned html, stores its size in html_size and returns a
// 'forgotten' unsafe pointer to the html, for returning to C
//
fn document_to_c(document: &NodeRef, info: &mut MorphInfo) -> u8 {
    let content = dom::serialize_html(document);
    return content_to_c(content, info);
}

fn content_to_c(content: Vec<u8>, info: &mut MorphInfo) -> u8 {
    info.size = content.len();

    let mut buf = content.into_boxed_slice();
    info.content = buf.as_mut_ptr();
    std::mem::forget(buf);
    1
}

fn c_string_to_str<'a>(s: *const u8) -> Result<&'a str, String> {
    return stringify_error(unsafe { CStr::from_ptr(s as *const i8) }.to_str());
}
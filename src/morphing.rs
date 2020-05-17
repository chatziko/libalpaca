//! Contains main morphing routines.
use std::ptr;
use std::path::Path;
use std::ffi::CStr;
use std::os::raw::c_char;

use pad::{get_html_padding, get_object_padding};
use dom;
use dom::{Object,ObjectKind};
use distribution::{Distributions, sample_ge, sample_ge_many};
use deterministic::*;

use kuchiki::NodeRef;

/// It samples a new page using probabilistic morphing, changes the
/// references to its objects accordingly, and pads it.
#[no_mangle]
pub extern "C" fn morph_html_Palpaca(
    html: *const c_char,
    root: *const c_char,
    html_path: *const c_char,
    dist_html: *const c_char,
    dist_obj_num: *const c_char,
    dist_obj_size: *const c_char,
    html_size: &mut usize,
    alias: usize,
) -> *const u8 {
    std::env::set_var("RUST_BACKTRACE", "full");

    // /* Convert arguments into &str */
    let cstr_html = unsafe { CStr::from_ptr(html) };
    let html = match cstr_html.to_str() {
        Ok(s) => s,
        Err(e) => {
            eprint!("libalpaca: cannot read html content: {}\n", e);
            return ptr::null();       // return NULL pointer if html cannot be converted to a string
        }
    };

    let cstr_root = unsafe { CStr::from_ptr(root) };
    let root = cstr_root.to_str().unwrap();

    let cstr_html_path = unsafe { CStr::from_ptr(html_path) };
    let html_path = cstr_html_path.to_str().unwrap();

    let cstr_dist_html = unsafe { CStr::from_ptr(dist_html) };
    let dist_html = cstr_dist_html.to_str().unwrap();

    let cstr_dist_obj_num = unsafe { CStr::from_ptr(dist_obj_num) };
    let dist_obj_num = cstr_dist_obj_num.to_str().unwrap();

    let cstr_dist_obj_size = unsafe { CStr::from_ptr(dist_obj_size) };
    let dist_obj_size = cstr_dist_obj_size.to_str().unwrap();

    let document = dom::parse_html(html);

    let mut objects = dom::parse_objects(&document, root, html_path, alias); // Vector of objects found in the html.
    let orig_n = objects.len(); // Number of original objects.

    // Construct a Distributions object containing the given distributions.
    let dists =  match Distributions::from(dist_html, dist_obj_num, dist_obj_size) {
        Ok(result) => result,
        Err(e) => {
            eprint!("libalpace: cannot load distributions: {}\n", e);
            return document_to_c(&document, html_size);
        }
    };

    match morph_from_distribution(&mut objects, &dists) {
        Ok(_) => {},
        Err(e) => {
            eprint!("libalpaca: morph_from_distribution failed: {}\n", e);
            return document_to_c(&document, html_size);
        }
    }

    match insert_objects_refs(&document, &objects, orig_n) {
        Ok(_) => {},
        Err(e) => {
            eprint!("libalpaca: insert_objects_refs failed: {}\n", e);
            return document_to_c(&document, html_size);
        }
    }

    // Sample the target HTML page size.
    let mut content = dom::serialize_html(&document);
    let html_min_size = content.len() + 7; // Plus 7 because of the comment characters.
    let target_size = match sample_ge(&dists.html, html_min_size) {
        Ok(size) => size,
        Err(e) => {
            eprint!("libalpaca: cannot sample html page size: {}\n", e);
            return document_to_c(&document, html_size);
        } 
    };

    get_html_padding(&mut content, target_size); // Pad the html to the target size.

    return content_to_c(content, html_size);
}

/// It samples a new page using deterministic morphing, changes the
/// references to its objects accordingly, and pads it.
#[no_mangle]
pub extern "C" fn morph_html_Dalpaca(
    html: *const c_char,
    root: *const c_char,
    html_path: *const c_char,
    obj_num: usize,
    obj_size: usize,
    max_obj_size: usize,
    html_size: &mut usize,
    alias: usize,
) -> *const u8 {
    std::env::set_var("RUST_BACKTRACE", "full");

    // /* Convert arguments into &str */
    let cstr_html = unsafe { CStr::from_ptr(html) };
    let html = match cstr_html.to_str() {
        Ok(s) => s,
        Err(e) => {
            eprint!("libalpaca: cannot read html content: {}\n", e);
            return ptr::null();       // return NULL pointer if html cannot be converted to a string
        }
    };

    let cstr_root = unsafe { CStr::from_ptr(root) };
    let root = cstr_root.to_str().unwrap();

    let cstr_html_path = unsafe { CStr::from_ptr(html_path) };
    let html_path = cstr_html_path.to_str().unwrap();

    let document = dom::parse_html(html);

    let mut objects = dom::parse_objects(&document, root, html_path, alias); // Vector of objects found in the html.
    let orig_n = objects.len(); // Number of original objects.

    match morph_deterministic(&mut objects, obj_num, obj_size, max_obj_size) {
        Ok(_) => {},
        Err(e) => {
            eprint!("libalpaca: cannot morph_deterministic: {}\n", e);
            return document_to_c(&document, html_size);
        },
    }

    // Insert the GET parameter to the objects.
    match insert_objects_refs(&document, &objects, orig_n) {
        Ok(_) => {},
        Err(e) => {
            eprint!("libalpaca: insert_objects_refs failed: {}\n", e);
            return document_to_c(&document, html_size);
        }
    }

    let mut content = dom::serialize_html(&document);
    let html_min_size = content.len() + 7; // Plus 7 because of the comment characters.
    let target_size = get_multiple(obj_size, html_min_size); // Target size for the html is a multiple of "obj_size".

    get_html_padding(&mut content, target_size); // Pad the html to the target size.

    return content_to_c(content, html_size);
}

/// Returns the object's padding.
#[no_mangle]
pub extern "C" fn morph_object(
    kind: *const c_char,
    query: *const c_char,
    size: &mut usize,
) -> *const u8 {

    let cstr_kind = unsafe { CStr::from_ptr(kind) };
    let kind = dom::parse_object_kind(cstr_kind.to_str().unwrap());

    let cstr_query = unsafe { CStr::from_ptr(query) };
    let query = cstr_query.to_str().unwrap();

    let target_size = dom::parse_target_size(query);
    if (target_size == 0) || (target_size <= *size) {
        // Target size has to be greater than current size.
        return content_to_c(Vec::new(), size);
    }

    let padding = get_object_padding(kind, *size, target_size); // Get the padding for the object.

    return content_to_c(padding, size);
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
    let target_count = sample_ge(&dists.obj_num, initial_obj_no)?;

    // To more closely match the actual obj_size distribution, we'll sample values for all objects,
    // And then we'll use the largest to pad existing objects and the smallest for padding objects.
    let mut target_sizes: Vec<usize> = sample_ge_many(&dists.obj_size, 1, target_count)?;
    target_sizes.sort_unstable();       // ascending

    // Pad existing objects
    for obj in &mut *objects {
        let needed_size = obj.content.len() +
            (if obj.kind == ObjectKind::CSS { 4 } else { 0 });   // CSS padding needs to be at least 4.

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

        eprint!("lala {} -> {}   {}\n", obj.content.len(), obj.target_size.unwrap_or(999), obj.uri);
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
        let mut min_size = objects[i].content.len();
        if objects[i].kind == ObjectKind::CSS {
            // CSS padding needs to be at least 4.
            min_size += 4;
        }

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

    let attr = match object.kind {
        ObjectKind::IMG => "src",
        ObjectKind::CSS => "href",
        _ => "",
    };

    let file_extension = Path::new(&object.uri).extension().unwrap().to_str().unwrap();

    // Check if there is already a GET parameter in the file path
    let prefix = if file_extension.contains("?") {
        '&'
    } else {
        '?'
    };

    new_link.insert(0, prefix);
    new_link.insert_str(0, &object.uri);

    dom::node_set_attribute(object.node.as_ref().unwrap(), attr, new_link);
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
        dom::node_set_attribute(&elem, "src", format!("__alpaca_fake_image.png?alpaca-padding={}", object.target_size.unwrap()));
        dom::node_set_attribute(&elem, "style", String::from("visibility:hidden"));
        node.append(elem);
    }
}

// Builds the returned html, stores its size in html_size and returns a
// 'forgotten' unsafe pointer to the html, for returning to C
//
fn document_to_c(document: &NodeRef, html_size: &mut usize) -> *const u8 {
    let content = dom::serialize_html(document);
    return content_to_c(content, html_size);
}

fn content_to_c(content: Vec<u8>, size: &mut usize) -> *const u8 {
    *size = content.len();

    let mut buf = content.into_boxed_slice();
    let data = buf.as_mut_ptr();
    std::mem::forget(buf);
    data
}
//! Contains main morphing routines.
use std::str;
use std::ptr;
use std::path::Path;
use std::ffi::CStr;
use std::os::raw::c_char;
use select::document::Document;
use select::predicate::Name;

use pad::{get_html_padding, get_object_padding};
use objects::*;
use parsing::{parse_objects, parse_target_size};
use distribution::{Distributions, sample_ge, sample_ge_many};
use deterministic::*;
use aux::*;

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

    let mut object = Object::from_str(html, "text/html", String::from(html_path)); // The html object.
    let mut objects = parse_objects(&object, root, html_path, alias); // Vector of objects found in the html.
    objects.sort_unstable_by(|a, b| a.content.len().cmp(&b.content.len()));

    let n = objects.len(); // Number of objects.

    // Construct a Distributions object containing the given distributions.
    let dists =  match Distributions::from(dist_html, dist_obj_num, dist_obj_size) {
        Ok(result) => result,
        Err(e) => {
            eprint!("libalpace: cannot load distributions: {}\n", e);
            *html_size = object.content.len();
            return object.as_ptr();
        }
    };

    // Try morphing for PAGE_SAMPLE_LIMIT times.
    match morph_from_distribution(&mut objects, n, &dists) {
        Ok(_) => {},
        Err(e) => {
            eprint!("libalpaca: morph_from_distribution failed: {}\n", e);
            *html_size = object.content.len();
            return object.as_ptr();
        }
    }

    match insert_objects_refs(&mut object, &objects, n) {
        Ok(_) => {},
        Err(e) => {
            eprint!("libalpaca: insert_objects_refs failed: {}\n", e);
            *html_size = object.content.len();
            return object.as_ptr();
        }
    }

    // Sample the target HTML page size.
    let html_min_size = object.content.len() + 7; // Plus 7 because of the comment characters.
    let target_size = match sample_ge(&dists.html, html_min_size) {
        Ok(size) => size,
        Err(e) => {
            eprint!("libalpaca: cannot sample html page size: {}\n", e);
            *html_size = object.content.len();
            return object.as_ptr();
        } 
    };

    get_html_padding(&mut object, target_size); // Pad the html to the target size.

    *html_size = object.content.len();
    object.as_ptr()
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

    let mut object = Object::from_str(html, "text/html", String::from(html_path)); // The html object.
    let mut objects = parse_objects(&object, root, html_path, alias); // Vector of objects found in the html.
    objects.sort_unstable_by(|a, b| a.content.len().cmp(&b.content.len()));

    let n = objects.len(); // Number of objects.

    match morph_deterministic(&mut objects, n, obj_num, obj_size, max_obj_size) {
        Ok(_) => {},
        Err(e) => {
            eprint!("libalpaca: cannot morph_deterministic: {}\n", e);
            *html_size = object.content.len();
            return object.as_ptr();
        },
    }

    // Insert the GET parameter to the objects.
    match insert_objects_refs(&mut object, &objects, n) {
        Ok(_) => {},
        Err(e) => {
            eprint!("libalpaca: insert_objects_refs failed: {}\n", e);
            *html_size = object.content.len();
            return object.as_ptr();
        }
    }

    let html_min_size = object.content.len() + 7; // Plus 7 because of the comment characters.
    let target_size = get_multiple(obj_size, html_min_size); // Target size for the html is a multiple of "obj_size".

    get_html_padding(&mut object, target_size); // Pad the html to the target size.

    *html_size = object.content.len();
    object.as_ptr()
}

/// Returns the object's padding.
#[no_mangle]
pub extern "C" fn morph_object(
    kind: *const c_char,
    query: *const c_char,
    size: &mut usize,
) -> *const u8 {

    let cstr_kind = unsafe { CStr::from_ptr(kind) };
    let kind = cstr_kind.to_str().unwrap();

    let cstr_query = unsafe { CStr::from_ptr(query) };
    let query = cstr_query.to_str().unwrap();


    let mut object = Object::from_str("", kind, String::new());
    let target_size = parse_target_size(query);

    if (target_size == 0) || (target_size <= *size) {
        // Target size has to be greater than current size.
        *size = 0;
        return object.as_ptr();
    }

    get_object_padding(&mut object, *size, target_size); // Get the padding for the object.

    *size = object.content.len(); // Update the size to contain the number of the padding bytes.
    object.as_ptr()
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
    min_count: usize,
    dists: &Distributions,
) -> Result<(), String> {
    // Sample target number of objects (count) and target sizes for morphed
    // objects.
    let target_count =  sample_ge(&dists.obj_num, min_count)?;

    let mut target_sizes: Vec<usize> = sample_ge_many(&dists.obj_size, 1, target_count)?;

    // Match target sizes to objects.
    // We will consider each target_size and decide whether to use it to pad
    // an object or to create a new object.
    // NOTE: We append newly created objects to the array objects.
    // NOTE: array objects is initially sorted.
    target_sizes.sort();

    let initial_obj_no = objects.len(); // Keep track of initial number of objects.
    let mut next_to_morph = 0; // Pointing at next object to morph.
    let mut create_new_obj;
    for s in target_sizes {
        create_new_obj = true;
        if (next_to_morph < initial_obj_no) && (s >= objects[next_to_morph].content.len()) {
            create_new_obj = false;
            if objects[next_to_morph].kind == ObjectKind::CSS && (objects[next_to_morph].content.len() + 4 > s) {
                // CSS padding needs to be at least 4.
                create_new_obj = true
            }
        }

        if !create_new_obj {
            // Pad i-th object to size s.
            objects[next_to_morph].target_size = Some(s);
            next_to_morph += 1;
        } else {
            // Create new padding object.
            let o = Object {
                kind: ObjectKind::Alpaca,
                content: Vec::new(),
                position: None,
                target_size: Some(s),
                uri: String::from("pad_object"),
            };
            objects.push(o);
        }
    }

    // No proper padding was found for some object. Continue without padding these objects, but print warning.
    if next_to_morph < initial_obj_no {
        let missing: Vec<&str> = objects[next_to_morph..initial_obj_no].into_iter().map(|o| o.uri.as_str()).collect();
        eprint!(
            "libalpaca: warning: no padding was found for the following objects:\n\t{}\n",
            missing.join("\n\t")
        );
    }

    Ok(())
}

fn morph_deterministic(
    objects: &mut Vec<Object>,
    min_count: usize,
    obj_num: usize,
    obj_size: usize,
    max_obj_size: usize,
) -> Result<(), String> {
    // Sample target number of objects (count) and target sizes for morphed
    // objects. Count is a multiple of "obj_num" and bigger than "min_count".
    // Target size for each objects is a multiple of "obj_size" and bigger
    // than the object's  original size.
    let target_count = get_multiple(obj_num, min_count);

    for i in 0..objects.len() {
        let mut min_size = objects[i].content.len();
        if objects[i].kind == ObjectKind::CSS {
            // CSS padding needs to be at least 4.
            min_size += 4;
        }

        let obj_target_size = get_multiple(obj_size, min_size);
        objects[i].target_size = Some(obj_target_size);
    }

    let fake_objects_count = target_count - min_count; // The number of fake objects.

    // To get the target size of each fake object, sample uniformly a multiple
    // of "obj_size" which is smaller than "max_obj_size".
    let fake_objects_sizes = get_multiples_in_range(obj_size, max_obj_size, fake_objects_count)?;

    // Add the fake objects to the vector.
    for i in 0..fake_objects_count {
        // Create new padding object.
        let o = Object {
            kind: ObjectKind::Alpaca,
            content: Vec::new(),
            position: None,
            target_size: Some(fake_objects_sizes[i]),
            uri: String::from("pad_object"),
        };
        objects.push(o);
    }

    Ok(())
}

/// Inserts the ALPaCA GET parameters to the html objects, and adds the fake objects to the html.
fn insert_objects_refs(html: &mut Object, objects: &[Object], n: usize) -> Result<(), String> {
    let init_obj = &objects[0..n]; // Slice which contains initial objects
    let padding_obj = &objects[n..]; // Slice which contains ALPaCA objects

    let html_str = stringify_error(str::from_utf8(&html.content))?; // Original html str
    let mut html_string = html_str.to_string(); // Original html String
    let mut document = Document::from(html_str);

    for object in init_obj {
        // ignore objects without target size
        if object.target_size.is_none() {
            continue;
        }

        let new_elem = append_ref(&document, &object);
        if new_elem == "" {
            continue;
        }
        // Replace the element in the html String
        let pos = object.position.unwrap();
        let elem = document.nth(pos).unwrap().html();

        html_string = html_string.replacen(&elem, &new_elem, 1);
    }

    // Update the document with the new html String
    document = Document::from(str::from_utf8(&html_string.into_bytes()).unwrap());
    // Add the fake ALPaCA objects
    html_string = add_padding_objects(&document, padding_obj);

    html_string.insert_str(0, "<!DOCTYPE html>");
    html.content = html_string.into_bytes();

    Ok(())

}

/// Appends the ALPaCA GET parameter to an html element
fn append_ref(document: &Document, object: &Object) -> String {
    // Construct the link with the appended new parameter
    let mut new_link = String::from("alpaca-padding=");
    new_link.push_str(&(object.target_size.unwrap().to_string())); // Append the target size

    let attr = match object.kind {
        ObjectKind::IMG => "src",
        ObjectKind::CSS => "href",
        _ => "",
    };

    let link = String::from(
        document
            .nth(object.position.unwrap())
            .unwrap()
            .attr(attr)
            .unwrap(),
    ); // Object's path
    let file_extension = Path::new(&link).extension().unwrap().to_str().unwrap();

    // Check if there is already a GET parameter in the file path
    let prefix = if file_extension.contains("?") {
        '&'
    } else {
        '?'
    };

    new_link.insert(0, prefix);
    new_link.insert_str(0, &link);

    let pos = object.position.unwrap();
    let element = document.nth(pos).unwrap().html();
    element.replace(&link, &new_link)
}

/// Adds the fake ALPaCA objects in the end of the html body
fn add_padding_objects(document: &Document, objects: &[Object]) -> String {
    // Find the document's node which corresponds to the html body
    for i in 0..document.nodes.len() {
        if document.nth(i).unwrap().name().is_none() {
            continue;
        }
        if document.nth(i).unwrap().name().unwrap() == "body" {
            let mut body = document.nth(i).unwrap().inner_html();
            for object in objects {
                let elem = format!(
                    "<img src=\"/__alpaca_fake_image.png?alpaca-padding={}\" style=\"visibility:hidden\">\n",
                    object.target_size.unwrap().to_string()
                );
                body.push_str(&elem);
            }
            body.insert_str(0, "<body>");
            body.push_str("</body>");

            // Return the new html which contains the padding objects in its body.
            return (document.find(Name("html")).next().unwrap().html())
                .replace(&document.nth(i).unwrap().html(), &body);

        }
    }

    // Return the original html if there was no node named body
    document.find(Name("html")).next().unwrap().html()
}
